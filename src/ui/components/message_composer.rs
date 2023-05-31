use adw;
use gtk::{
    self,
    traits::{ButtonExt, GridExt, GtkWindowExt, OrientableExt, TextViewExt, WidgetExt},
};
use relm4::{
    component::{AsyncComponent, AsyncComponentParts},
    AsyncComponentSender, RelmWidgetExt,
};

pub struct MessageComposer {}

#[derive(Debug)]
pub enum MessageComposerInput {
    CancelButtonClicked,
}

#[relm4::component(pub async)]
impl AsyncComponent for MessageComposer {
    type Input = MessageComposerInput;
    type Output = ();
    type Init = ();
    type CommandOutput = ();

    view! {
        #[root]
        adw::ApplicationWindow {
            set_default_size: (800, 600),
            set_title = Some(""),

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                adw::HeaderBar {
                    set_centering_policy: adw::CenteringPolicy::Strict,
                    set_show_end_title_buttons: false,
                    pack_start = &gtk::Button {
                        set_label: "Cancel",
                        connect_clicked => MessageComposerInput::CancelButtonClicked
                    },

                    pack_end = &gtk::Button {
                        set_label: "Send",
                    }
                },

                gtk::Grid {
                    set_margin_all: 10,
                    attach[0, 0, 2, 1] = &gtk::Label {
                        set_label: "From"
                    },
                    attach[3,0,1,1] = &gtk::DropDown {
                        set_hexpand: true
                    },
                    attach[0,1,2,1] = &gtk::Label {
                        set_label: "To"
                    },
                    attach[3,1,1,1] = &gtk::Entry {},
                    attach[0,2,2,1] = &gtk::Label {
                        set_label: "Subject"
                    },
                    attach[3,2,1,1] = &gtk::Entry {},
                    set_column_spacing: 10,
                    set_row_spacing: 10,
                },
                gtk::Frame {
                    inline_css: "border-radius: 0px",
                    gtk::TextView {
                        set_left_margin: 5,
                        set_right_margin: 5,
                        set_top_margin: 5,
                        set_bottom_margin: 5,

                        set_editable: true,
                        set_monospace: true,
                        set_hexpand: true,
                        set_vexpand: true
                    }
                }
            }
        }
    }

    async fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let model = MessageComposer {};
        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            MessageComposerInput::CancelButtonClicked => root.close(),
        }
    }
}
