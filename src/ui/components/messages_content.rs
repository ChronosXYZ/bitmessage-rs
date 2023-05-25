use gtk::traits::{OrientableExt, WidgetExt};
use relm4::{
    component::{AsyncComponent, AsyncComponentParts},
    loading_widgets::LoadingWidgets,
    view, AsyncComponentSender, RelmWidgetExt,
};

use super::{
    messages_sidebar::SelectedFolder,
    utils::typed_list_view::{RelmListItem, TypedListView},
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct MessagesListItem {
    title: String,
    date: chrono::NaiveDateTime,
    from: String,
}

struct MessagesListItemWidgets {
    label: gtk::Label,
}

impl RelmListItem for MessagesListItem {
    type Root = gtk::Box;
    type Widgets = MessagesListItemWidgets;

    fn setup(list_item: &gtk::ListItem, column_index: usize) -> (Self::Root, Self::Widgets) {
        view! {
            #[name(root)]
            gtk::Box{
                #[name(label)]
                gtk::Label {}
            }
        }

        let widgets = Self::Widgets { label };
        (root, widgets)
    }
}

pub struct MessagesContent {
    selected_folder: Option<SelectedFolder>,
    messages_list_view: TypedListView<MessagesListItem, gtk::SingleSelection, gtk::ColumnView>,
}

#[derive(Debug)]
pub enum MessagesContentInput {
    FolderSelected(SelectedFolder),
}

#[relm4::component(pub async)]
impl AsyncComponent for MessagesContent {
    type Init = ();
    type Input = MessagesContentInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_hexpand: true,
            match model.selected_folder.clone() {
                Some(_) => {
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        #[local_ref]
                        messages_list -> gtk::ColumnView {}
                    }
                },
                None => {
                    gtk::Label {
                        set_vexpand: true,
                        set_label: "Select folder to view messages",
                        add_css_class: "large-title"
                    }
                }
            }
        }
    }

    fn init_loading_widgets(root: &mut Self::Root) -> Option<LoadingWidgets> {
        view! {
                #[local_ref]
                root {
                    #[name(loading)]
                    gtk::CenterBox {
                        set_margin_all: 100,
                        set_orientation: gtk::Orientation::Vertical,
                        #[wrap(Some)]
                        set_center_widget = &gtk::Spinner {
                            start: (),
                            set_size_request: (40, 40),
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                        },
                    }
                }
        }
        Some(LoadingWidgets::new(root, loading))
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let messages_list_view = TypedListView::with_sorting_col(vec![
            "Date".to_string(),
            "From".to_string(),
            "Title".to_string(),
        ]);

        let model = Self {
            selected_folder: None,
            messages_list_view,
        };
        let messages_list = &model.messages_list_view.view;

        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        _sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            MessagesContentInput::FolderSelected(selected_folder) => {
                self.selected_folder = Some(selected_folder);
            }
        }
    }
}
