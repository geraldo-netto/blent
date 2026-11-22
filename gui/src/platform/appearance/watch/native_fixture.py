"""T1795: isolated GTK changes and original signal/source unregistration."""
import runpy
import sys
import gi

gi.require_version("Gtk", "3.0")
from gi.repository import Gtk, Pango, Gio, GLib

probe = runpy.run_path(sys.argv[1])
seen = []


def change():
    settings = Gtk.Settings.get_default()
    settings.set_property("gtk-font-name", "Sans 14")
    settings.set_property("gtk-theme-name", "HighContrast")
    settings.set_property("gtk-enable-animations", False)
    scale = probe["scale_settings"](Gio)
    assert scale is not None
    scale.set_double("text-scaling-factor", 1.5)
    return False


def emit(value):
    seen.append(value)
    if len(seen) == 1:
        GLib.idle_add(change)
    else:
        watch.loop.quit()


def expired():
    watch.loop.quit()
    return False


watch = probe["Watch"](Gtk, Pango, Gio, GLib, emit)
expiry = GLib.timeout_add(3000, expired)
try:
    watch.start()
    watch.loop.run()
finally:
    watch.close()
assert len(seen) == 2, seen
GLib.source_remove(expiry)
assert seen[1]["font_points"] == 14.0, seen
assert seen[1]["source"] == "GTK HighContrast", seen
assert seen[1]["text_scale"] == 1.5, seen
assert seen[1]["animations"] is False, seen
assert seen[1]["background"] != seen[0]["background"], seen
assert not watch.connections and watch.pending == watch.input_source == 0
watch.changed()
assert watch.pending == 0
print("T1795 GTK font/theme/scale/animation notifications and cleanup passed")
