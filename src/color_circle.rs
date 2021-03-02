use cascade::cascade;
use glib::subclass;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::cell::Cell;
use std::f64::consts::PI;
use std::ptr;

use crate::color::Hs;

#[derive(Default)]
pub struct ColorCircleInner {
    hs: Cell<Hs>,
    alpha: Cell<f64>,
    symbol: Cell<&'static str>,
    mouseover: Cell<bool>,
}

impl ObjectSubclass for ColorCircleInner {
    const NAME: &'static str = "S76ColorCircle";

    type ParentType = gtk::Button;
    type Type = ColorCircle;
    type Interfaces = ();

    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib::object_subclass!();

    fn new() -> Self {
        Self {
            alpha: Cell::new(1.),
            ..Default::default()
        }
    }
}

impl ObjectImpl for ColorCircleInner {
    fn constructed(&self, widget: &ColorCircle) {
        self.parent_constructed(widget);

        widget.connect_enter_notify_event(|widget, _| {
            widget.inner().mouseover.set(true);
            widget.queue_draw();
            Inhibit(false)
        });

        widget.connect_leave_notify_event(|widget, _| {
            widget.inner().mouseover.set(false);
            widget.queue_draw();
            Inhibit(false)
        });

        widget.add_events(gdk::EventMask::POINTER_MOTION_MASK);
    }

    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![glib::ParamSpec::boxed(
                "hs",
                "hs",
                "hs",
                Hs::get_type(),
                glib::ParamFlags::READWRITE,
            )]
        });

        PROPERTIES.as_ref()
    }

    fn set_property(
        &self,
        widget: &ColorCircle,
        _id: usize,
        value: &glib::Value,
        pspec: &glib::ParamSpec,
    ) {
        match pspec.get_name() {
            "hs" => {
                let hs: &Hs = value.get_some().unwrap();
                widget.set_hs(*hs);
            }
            _ => unimplemented!(),
        }
    }

    fn get_property(
        &self,
        widget: &ColorCircle,
        _id: usize,
        pspec: &glib::ParamSpec,
    ) -> glib::Value {
        match pspec.get_name() {
            "hs" => widget.hs().to_value(),
            _ => unimplemented!(),
        }
    }
}

impl WidgetImpl for ColorCircleInner {
    fn draw(&self, widget: &ColorCircle, cr: &cairo::Context) -> Inhibit {
        let width = f64::from(widget.get_allocated_width());
        let height = f64::from(widget.get_allocated_height());

        let style = widget.get_style_context();
        let fg = style.get_color(gtk::StateFlags::NORMAL);

        let radius = width.min(height) / 2.;
        let (r, g, b) = widget.hs().to_rgb().to_floats();
        let alpha = self.alpha.get();

        cr.arc(radius, radius, radius, 0., 2. * PI);
        cr.set_source_rgba(r, g, b, alpha);
        cr.fill_preserve();
        if self.mouseover.get() {
            cr.set_source_rgba(0., 0., 0., 0.2);
            cr.fill_preserve();
        }
        cr.set_line_width(1.);
        cr.set_source_rgb(0.5, 0.5, 0.5);
        cr.stroke();

        let text = self.symbol.get();
        let attrs = cascade! {
            pango::AttrList::new();
            ..insert(pango::Attribute::new_size(14 * pango::SCALE));
        };
        let layout = cascade! {
            widget.create_pango_layout(Some(text));
            ..set_width((width * pango::SCALE as f64) as i32);
            ..set_alignment(pango::Alignment::Center);
            ..set_attributes(Some(&attrs));
        };
        let text_height = layout.get_pixel_size().1 as f64;
        cr.new_path();
        cr.move_to(0., (height - text_height) / 2.);
        cr.set_source_rgb(fg.red, fg.green, fg.blue);
        pangocairo::show_layout(cr, &layout);

        cr.stroke();

        Inhibit(false)
    }

    fn motion_notify_event(&self, widget: &ColorCircle, evt: &gdk::EventMotion) -> Inhibit {
        let width = f64::from(widget.get_allocated_width());
        let height = f64::from(widget.get_allocated_height());
        let radius = width.min(height) / 2.;
        let (x, y) = evt.get_position();

        let mouseover = (x - radius).hypot(y - radius) < radius;
        if self.mouseover.replace(mouseover) != mouseover {
            widget.queue_draw();
        }

        Inhibit(false)
    }
}

impl ContainerImpl for ColorCircleInner {}
impl BinImpl for ColorCircleInner {}
impl ButtonImpl for ColorCircleInner {}

glib::wrapper! {
    pub struct ColorCircle(ObjectSubclass<ColorCircleInner>)
        @extends gtk::Button, gtk::Bin, gtk::Container, gtk::Widget;
}

impl ColorCircle {
    pub fn new(size: i32) -> Self {
        let color_circle: Self = glib::Object::new(&[]).unwrap();

        color_circle.set_size_request(size, size);

        color_circle
    }

    fn inner(&self) -> &ColorCircleInner {
        ColorCircleInner::from_instance(self)
    }

    pub fn set_hs(&self, color: Hs) {
        self.inner().hs.set(color);
        self.notify("hs");
        self.queue_draw();
    }

    pub fn hs(&self) -> Hs {
        self.inner().hs.get()
    }

    pub fn set_symbol(&self, symbol: &'static str) {
        self.inner().symbol.set(symbol);
        self.queue_draw();
    }

    pub fn set_alpha(&self, alpha: f64) {
        self.inner().alpha.set(alpha);
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        ptr::eq(self.inner(), other.inner())
    }
}
