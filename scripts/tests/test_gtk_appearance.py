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
