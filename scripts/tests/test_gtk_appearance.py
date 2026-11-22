"""T1786: embedded native appearance probe, including partial-failure cleanup."""
import contextlib
import importlib.util
import io
from pathlib import Path
from types import SimpleNamespace as NS
import sys
import unittest
from unittest.mock import Mock, patch

SOURCE = Path(__file__).resolve().parents[2] / 'gui/src/platform/appearance/gtk.py'
SPEC = importlib.util.spec_from_file_location('blent_gtk_appearance', SOURCE)
PROBE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROBE)


class GtkAppearanceTest(unittest.TestCase):
    def fixture(self):
        rgba = NS(red=0.2, green=0.4, blue=0.6, alpha=1.0)
        context = NS(lookup_color=Mock(return_value=(True, rgba)))
        window = NS(get_style_context=lambda: context, destroy=Mock())
        values = {'gtk-font-name': 'Sans 10', 'gtk-theme-name': 'Fixture', 'gtk-enable-animations': False}
        settings = NS(set_property=Mock(), get_property=values.__getitem__)
        gtk = NS(init_check=lambda _: (True, []), Settings=NS(get_default=lambda: settings), Window=lambda: window)
        font = NS(get_family=lambda: 'Sans', get_size=lambda: 10240)
        pango = NS(SCALE=1024, FontDescription=NS(from_string=lambda _: font))
        gio = NS(SettingsSchemaSource=NS(get_default=lambda: None))
        return gtk, pango, gio, window, context

    def test_t1786_snapshot_releases_window_on_success_and_failure(self):
        gtk, pango, gio, window, context = self.fixture()
        result = PROBE.snapshot(gtk, pango, gio)
        self.assertEqual(result['background'], [51, 102, 153])
        self.assertEqual(result['font_points'], 10.0)
        window.destroy.assert_called_once()
        context.lookup_color.return_value = (False, None)
        with self.assertRaises(ValueError):
            PROBE.snapshot(gtk, pango, gio)
        self.assertEqual(window.destroy.call_count, 2)
        gtk.init_check = lambda _: (False, [])
        with self.assertRaises(ValueError):
            PROBE.snapshot(gtk, pango, gio)
        self.assertEqual(window.destroy.call_count, 2)

    def test_t1786_scale_schema_absence_and_both_desktop_schemas(self):
        for index in [0, 1, 2]:
            schema = NS(has_key=lambda _: True)
            source = NS(lookup=Mock(side_effect=[None] * index + [schema]))
            gio = NS(SettingsSchemaSource=NS(get_default=lambda: source),
                     Settings=NS(new_full=lambda *_: NS(get_double=lambda _: 1.5)))
            self.assertEqual(PROBE.text_scale(gio), 1.5 if index < 2 else 1.0)

    def test_t1786_main_serializes_only_snapshot_values(self):
        gtk, pango, gio, window, _ = self.fixture()
        gi = NS(require_version=Mock())
        repository = NS(Gtk=gtk, Pango=pango, Gio=gio)
        output = io.StringIO()
        with patch.dict(sys.modules, {'gi': gi, 'gi.repository': repository}), contextlib.redirect_stdout(output):
            PROBE.main()
        self.assertIn('source = "GTK Fixture"', output.getvalue())
        self.assertIn('animations = false', output.getvalue())
        window.destroy.assert_called_once()

    def test_t1786_alpha_is_composited_against_the_native_surface(self):
        context = NS(lookup_color=lambda _: (True, NS(red=0, green=0, blue=0, alpha=0.5)))
        self.assertEqual(PROBE.rgb(context, 'foreground', [200, 100, 0]), [100, 50, 0])


class GtkWatchTest(unittest.TestCase):
    def watcher(self):
        gtk, pango, gio, window, context = GtkAppearanceTest.fixture(self)
        settings = gtk.Settings.get_default()
        settings.connect = Mock(return_value=17)
        settings.disconnect = Mock()
        sources = []
        loop = NS(run=Mock(), quit=Mock())
        glib = NS(MainLoop=lambda: loop, timeout_add=Mock(return_value=23),
                  io_add_watch=Mock(return_value=24), source_remove=sources.append,
                  IO_IN=1, IO_HUP=2, IO_ERR=4)
        emit = Mock()
        watch = PROBE.Watch(gtk, pango, gio, glib, emit)
        return watch, settings, glib, emit, context, sources

    def test_t1795_changes_coalesce_and_close_disconnects_original_handlers(self):
        watch, settings, glib, emit, _, sources = self.watcher()
        watch.start()
        for _ in range(100):
            watch.changed()
        glib.timeout_add.assert_called_once()
        self.assertFalse(watch.refresh())
        emit.assert_called_once()
        watch.changed()
        watch.input_source = 24
        watch.close()
        self.assertEqual(settings.disconnect.call_count, 4)
        self.assertEqual(sources, [23, 24])
        watch.changed()
        self.assertEqual(watch.pending, 0)

    def test_t1795_snapshot_failure_stops_loop_and_run_unregisters(self):
        watch, settings, _, emit, context, sources = self.watcher()
        context.lookup_color.return_value = (False, None)
        watch.start()
        self.assertFalse(watch.refresh())
        self.assertIn('error', emit.call_args.args[0])
        self.assertIsNone(watch.error)
        watch.loop.quit.assert_not_called()
        emit.side_effect = BrokenPipeError('consumer gone')
        self.assertFalse(watch.refresh())
        self.assertIsInstance(watch.error, BrokenPipeError)
        watch.loop.quit.assert_called_once()
        watch.close()
        watch, settings, _, _, _, sources = self.watcher()
        watch.run()
        self.assertEqual(sources, [23, 24])
        self.assertFalse(watch.connections)
        self.assertFalse(watch.input_closed())
        watch.error = ValueError('rejected')
        with self.assertRaises(ValueError):
            watch.run()

    def test_t1795_binary_snapshot_limit_and_partial_registration_failure(self):
        output = NS(buffer=io.BytesIO())
        with patch.object(PROBE.sys, 'stdout', output):
            PROBE.emit_frame({'source': 'Fixture'})
            with self.assertRaises(ValueError):
                PROBE.emit_frame({'source': 'x' * 8193})
        data = output.buffer.getvalue()
        self.assertEqual(int.from_bytes(data[:4], 'big'), len(data) - 4)
        watch, settings, _, _, _, _ = self.watcher()
        settings.connect.side_effect = [17, RuntimeError('registration failed')]
        with self.assertRaises(RuntimeError):
            watch.run()
        settings.disconnect.assert_called_once_with(17)

    def test_t1795_scale_notifications_arm_after_registration_and_main_runs_watch(self):
        watch, _, glib, _, _, _ = self.watcher()
        scale = NS(connect=Mock(return_value=18), disconnect=Mock(), get_double=Mock(return_value=1.0))
        with patch.object(PROBE, 'scale_settings', return_value=scale):
            watch.start()
        scale.connect.assert_called_once()
        scale.get_double.assert_called_once()
        watch.close()
        scale.disconnect.assert_called_once_with(18)
        gtk, pango, gio = watch.modules
        gi = NS(require_version=Mock())
        repository = NS(Gtk=gtk, Pango=pango, Gio=gio, GLib=glib)
        with patch.dict(sys.modules, {'gi': gi, 'gi.repository': repository}), patch.object(PROBE.sys, 'argv', ['probe', '--watch']), patch.object(PROBE, 'Watch') as factory:
            PROBE.main()
        factory.return_value.run.assert_called_once()
