"""Process-local GTK light appearance; never write desktop preferences."""
import json
import struct
import sys


def rgb(context, name, background):
    found, color = context.lookup_color(name)
    if not found:
        raise ValueError("GTK theme does not expose " + name)
    channels = (color.red, color.green, color.blue)
    return [round(255 * value * color.alpha + base * (1 - color.alpha))
            for value, base in zip(channels, background)]


def scale_settings(Gio):
    source = Gio.SettingsSchemaSource.get_default()
    if source is None:
        return None
    for name in ("org.cinnamon.desktop.interface", "org.gnome.desktop.interface"):
        schema = source.lookup(name, True)
        if schema and schema.has_key("text-scaling-factor"):
            return Gio.Settings.new_full(schema, None, None)
    return None


def text_scale(Gio):
    settings = scale_settings(Gio)
    return settings.get_double("text-scaling-factor") if settings else 1.0


def snapshot(Gtk, Pango, Gio):
    if not Gtk.init_check([])[0]:
        raise ValueError("GTK display unavailable")
    settings = Gtk.Settings.get_default()
    settings.set_property("gtk-application-prefer-dark-theme", False)
    window = Gtk.Window()
    try:
        context = window.get_style_context()
        background = rgb(context, "theme_bg_color", [255, 255, 255])
        font = Pango.FontDescription.from_string(settings.get_property("gtk-font-name"))
        return dict(source="GTK " + settings.get_property("gtk-theme-name"),
                    background=background, surface=rgb(context, "theme_base_color", background),
                    foreground=rgb(context, "theme_fg_color", background),
                    accent=rgb(context, "theme_selected_bg_color", background),
                    selected_text=rgb(context, "theme_selected_fg_color", background),
                    font_family=font.get_family(), font_points=font.get_size() / Pango.SCALE,
                    text_scale=text_scale(Gio), animations=settings.get_property("gtk-enable-animations"))
    finally:
        window.destroy()


class Watch:
    """Fixed signal registrations and one coalesced one-shot refresh source."""
    def __init__(self, Gtk, Pango, Gio, GLib, emit):
        self.modules = Gtk, Pango, Gio
        self.glib = GLib
        self.emit = emit
        self.pending = 0
        self.connections = []
        self.closed = False
        self.loop = GLib.MainLoop()
        self.error = None
        self.input_source = 0

    def connect(self, owner, signal):
        self.connections.append((owner, owner.connect(signal, self.changed)))

    def start(self):
        Gtk, _, Gio = self.modules
        if not Gtk.init_check([])[0]:
            raise ValueError("GTK display unavailable")
        settings = Gtk.Settings.get_default()
        for name in ("gtk-theme-name", "gtk-font-name", "gtk-xft-dpi", "gtk-enable-animations"):
            self.connect(settings, "notify::" + name)
        scale = scale_settings(Gio)
        if scale:
            self.connect(scale, "changed::text-scaling-factor")
            scale.get_double("text-scaling-factor")  # Arm GSettings notifications.
        self.changed()

    def changed(self, *_):
        if not self.closed and not self.pending:
            self.pending = self.glib.timeout_add(100, self.refresh)

    def refresh(self):
        self.pending = 0
        try:
            value = snapshot(*self.modules)
        except Exception:
            value = {"error": "GTK appearance snapshot unavailable"}
        try:
            self.emit(value)
        except Exception as error:
            self.error = error
            self.loop.quit()
        return False

    def input_closed(self, *_):
        self.input_source = 0
        self.loop.quit()
        return False

    def close(self):
        self.closed = True
        for source in (self.pending, self.input_source):
            if source:
                self.glib.source_remove(source)
        self.pending = self.input_source = 0
        for owner, handler in self.connections:
            owner.disconnect(handler)
        self.connections.clear()

    def run(self):
        try:
            self.start()
            self.input_source = self.glib.io_add_watch(
                0, self.glib.IO_IN | self.glib.IO_HUP | self.glib.IO_ERR, self.input_closed)
            self.loop.run()
        finally:
            self.close()
        if self.error:
            raise self.error


def emit_frame(values):
    data = "\n".join(key + " = " + json.dumps(value) for key, value in values.items()).encode()
    if len(data) > 8192:
        raise ValueError("GTK appearance exceeds frame limit")
    sys.stdout.buffer.write(struct.pack(">I", len(data)) + data)
    sys.stdout.buffer.flush()


def main():
    import gi
    gi.require_version("Gtk", "3.0")
    from gi.repository import Gtk, Pango, Gio
    if sys.argv[1:] == ["--watch"]:
        from gi.repository import GLib
        Watch(Gtk, Pango, Gio, GLib, emit_frame).run()
    else:
        for key, value in snapshot(Gtk, Pango, Gio).items():
            print(key + " = " + json.dumps(value))


if __name__ == "__main__":
    main()
