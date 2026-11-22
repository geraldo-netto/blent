"""One process-local GTK light snapshot; never write desktop preferences."""
import json


def rgb(context, name, background):
    found, color = context.lookup_color(name)
    if not found:
        raise ValueError("GTK theme does not expose " + name)
    channels = (color.red, color.green, color.blue)
    return [round(255 * value * color.alpha + base * (1 - color.alpha))
            for value, base in zip(channels, background)]


def text_scale(Gio):
    source = Gio.SettingsSchemaSource.get_default()
    if source is None:
        return 1.0
    for name in ("org.cinnamon.desktop.interface", "org.gnome.desktop.interface"):
        schema = source.lookup(name, True)
        if schema and schema.has_key("text-scaling-factor"):
            return Gio.Settings.new_full(schema, None, None).get_double("text-scaling-factor")
    return 1.0


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


def main():
    import gi
    gi.require_version("Gtk", "3.0")
    from gi.repository import Gtk, Pango, Gio
    for key, value in snapshot(Gtk, Pango, Gio).items():
        print(key + " = " + json.dumps(value))


if __name__ == "__main__":
    main()
