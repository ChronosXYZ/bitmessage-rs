use std::cell::{Ref, RefMut};

use gtk::{
    self, gio,
    glib::BoxedAnyObject,
    prelude::{Cast, CastNone, ObjectExt, StaticType},
    traits::WidgetExt,
};
use relm4::{
    component::{AsyncComponentParts, SimpleAsyncComponent},
    view, AsyncComponentSender,
};

use crate::ui::state;

use super::utils::typed_list_view::RelmListItem;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct FolderItem {
    label: String,
    is_top_level: bool,
}

struct IdentityItemWidgets {
    expander: gtk::TreeExpander,
    label: gtk::Label,
}

impl RelmListItem for FolderItem {
    type Root = gtk::TreeExpander;
    type Widgets = IdentityItemWidgets;

    fn setup(list_item: &gtk::ListItem, column_index: usize) -> (Self::Root, Self::Widgets) {
        view! {
            #[name(expander)]
            gtk::TreeExpander {
                #[name(label)]
                #[wrap(Some)]
                set_child = &gtk::Label {}
            }
        }

        let widgets = IdentityItemWidgets {
            expander: expander.clone(),
            label,
        };
        (expander, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, root: &mut Self::Root, _column_index: usize) {
        widgets.label.set_text(&self.label);
    }
}

pub struct MessagesSidebar {
    tree_model: gtk::TreeListModel,
    list_view: gtk::ListView,
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for MessagesSidebar {
    type Init = ();
    type Input = ();
    type Output = ();

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_width_request: 200,
            #[local_ref]
            list_view -> gtk::ListView {}
        }
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let root_store = gio::ListStore::new(BoxedAnyObject::static_type());

        let identities = state::STATE
            .write_inner()
            .client
            .as_mut()
            .unwrap()
            .get_own_identities()
            .await;
        for i in identities {
            root_store.append(&BoxedAnyObject::new(FolderItem {
                label: format!(
                    "{} ({})",
                    if i.label.is_empty() {
                        "No label"
                    } else {
                        i.label.as_str()
                    },
                    i.string_repr
                ),
                is_top_level: true,
            }))
        }

        let tree_model = gtk::TreeListModel::new(root_store.clone(), false, true, |o| {
            let boxed_object = o.clone().downcast::<BoxedAnyObject>().unwrap();
            let item: Ref<FolderItem> = boxed_object.borrow();
            if item.is_top_level {
                let inner_folders = gio::ListStore::new(BoxedAnyObject::static_type());
                inner_folders.append(&BoxedAnyObject::new(FolderItem {
                    label: "inbox".to_string(),
                    is_top_level: false,
                }));
                inner_folders.append(&BoxedAnyObject::new(FolderItem {
                    label: "sent".to_string(),
                    is_top_level: false,
                }));
                return Some(inner_folders.upcast());
            }
            None
        });

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let (root, widgets) = FolderItem::setup(item, 0);
            unsafe { root.set_data("widgets", widgets) };
            item.set_child(Some(&root));
        });

        factory.connect_bind(move |_factory, item| {
            let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let widget = list_item.downcast_ref::<gtk::ListItem>().unwrap().child();

            let list_row = list_item
                .item()
                .unwrap()
                .downcast::<gtk::TreeListRow>()
                .unwrap();
            let obj = list_row
                .item()
                .unwrap()
                .downcast::<BoxedAnyObject>()
                .unwrap();
            let mut obj: RefMut<FolderItem> = obj.borrow_mut();
            let mut root = widget
                .and_downcast::<<FolderItem as RelmListItem>::Root>()
                .unwrap();

            let mut widgets = unsafe { root.steal_data("widgets") }.unwrap();
            obj.bind(&mut widgets, &mut root, 0);
            widgets.expander.set_list_row(Some(&list_row));
            unsafe { root.set_data("widgets", widgets) };
        });

        let selection_model = gtk::SingleSelection::new(Some(tree_model.clone()));
        let list_view = gtk::ListView::new(Some(selection_model.clone()), Some(factory));

        let model = Self {
            list_view: list_view.clone(),
            tree_model,
        };

        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }
}
