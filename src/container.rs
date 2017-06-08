/*
 * Copyright (c) 2017 Boucher, Antoni <bouanto@zoho.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
 * FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
 * COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
 * IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

use gtk;
use gtk::{ContainerExt, IsA, Object, WidgetExt};

use super::{Component, DisplayVariant, Relm, create_component, create_widget, init_component};
use widget::Widget;

/// Struct for relm containers to add GTK+ and relm `Widget`s.
pub struct ContainerComponent<WIDGET: Container + Widget> {
    component: Component<WIDGET>,
    /// The default container of this component.
    pub container: WIDGET::Container,
    /// Additional containers used for multi-containers. This can be () if not needed.
    pub containers: WIDGET::Containers,
}

impl<WIDGET: Container + Widget> ContainerComponent<WIDGET> {
    #[doc(hidden)]
    pub fn new(component: Component<WIDGET>, container: WIDGET::Container, containers: WIDGET::Containers) -> Self {
        ContainerComponent {
            component,
            container,
            containers,
        }
    }

    /// Add a GTK+ widget to a relm container.
    pub fn add<CHILDWIDGET: IsA<gtk::Widget>>(&self, widget: &CHILDWIDGET) {
        self.container.add(widget);
    }

    /// Add a relm widget to a relm container.
    pub fn add_widget<CHILDWIDGET, PARENTWIDGET>(&self, relm: &Relm<PARENTWIDGET>, model_param: CHILDWIDGET::ModelParam)
        -> Component<CHILDWIDGET>
        where CHILDWIDGET: Widget + 'static,
              PARENTWIDGET: Widget,
              WIDGET::Container: ContainerExt + IsA<gtk::Widget> + IsA<Object>,
    {
        let component = create_component::<CHILDWIDGET, _>(relm, model_param);
        WIDGET::add_widget(&self, &component);
        component
    }

    // TODO: add delete methods?

    /// Get the widget of the component.
    pub fn widget(&self) -> &WIDGET::Root {
        self.component.widget()
    }
}

/// Trait to implement relm container widget.
pub trait Container: Widget {
    /// The type of the containing widget, i.e. where the child widgets will be added.
    type Container: Clone + IsA<gtk::Container> + IsA<Object> + IsA<gtk::Widget>;
    /// Type to contain the additional container widgets.
    // TODO: put that in yet another trait?
    type Containers;

    /// Add a relm widget to this container.
    fn add_widget<WIDGET: Widget>(container: &ContainerComponent<Self>, component: &Component<WIDGET>) {
        container.container.add(component.widget());
    }

    /// Get the containing widget, i.e. the widget where the children will be added.
    fn container(&self) -> &Self::Container;

    /// Get additional container widgets.
    /// This is useful to create a multi-container.
    fn other_containers(&self) -> Self::Containers;
}

/// Extension trait for GTK+ containers to add and remove relm `Widget`s.
pub trait ContainerWidget {
    /// Add a relm `Container` to the current GTK+ container.
    ///
    /// # Note
    ///
    /// The returned `ContainerComponent` must be stored in a `Widget`. If it is not stored, a
    /// communication receiver will be droped which will cause events to be ignored for this
    /// widget.
    fn add_container<CHILDWIDGET, WIDGET>(&self, relm: &Relm<WIDGET>, model_param: CHILDWIDGET::ModelParam)
            -> ContainerComponent<CHILDWIDGET>
        where CHILDWIDGET: Container + Widget + 'static,
              CHILDWIDGET::Msg: DisplayVariant + 'static,
              CHILDWIDGET::Root: IsA<gtk::Widget> + IsA<Object> + WidgetExt,
              WIDGET: Widget;

    /// Add a relm `Widget` to the current GTK+ container.
    ///
    /// # Note
    ///
    /// The returned `Component` must be stored in a `Widget`. If it is not stored, a communication
    /// receiver will be droped which will cause events to be ignored for this widget.
    fn add_widget<CHILDWIDGET, WIDGET>(&self, relm: &Relm<WIDGET>, model_param: CHILDWIDGET::ModelParam)
            -> Component<CHILDWIDGET>
        where CHILDWIDGET: Widget + 'static,
              CHILDWIDGET::Msg: DisplayVariant + 'static,
              CHILDWIDGET::Root: IsA<gtk::Widget> + IsA<Object> + WidgetExt,
              WIDGET: Widget;

    /// Remove a relm `Widget` from the current GTK+ container.
    fn remove_widget<CHILDWIDGET>(&self, component: Component<CHILDWIDGET>)
        where CHILDWIDGET: Widget,
              CHILDWIDGET::Root: IsA<gtk::Widget>;
}

impl<W: Clone + ContainerExt + IsA<gtk::Widget> + IsA<Object>> ContainerWidget for W {
    fn add_container<CHILDWIDGET, WIDGET>(&self, relm: &Relm<WIDGET>, model_param: CHILDWIDGET::ModelParam)
            -> ContainerComponent<CHILDWIDGET>
        where CHILDWIDGET: Container + Widget + 'static,
              CHILDWIDGET::Msg: DisplayVariant + 'static,
              CHILDWIDGET::Root: IsA<gtk::Widget> + IsA<Object> + WidgetExt,
              WIDGET: Widget,
    {
        let (widget, component, child_relm) = create_widget::<CHILDWIDGET>(relm.context(), model_param);
        let container = component.container().clone();
        let containers = component.other_containers();
        let root = component.root().clone();
        self.add(&root);
        component.on_add(self.clone());
        init_component::<CHILDWIDGET>(widget.stream(), component, relm.context(), &child_relm);
        ContainerComponent::new(widget, container, containers)
    }

    fn add_widget<CHILDWIDGET, WIDGET>(&self, relm: &Relm<WIDGET>, model_param: CHILDWIDGET::ModelParam)
            -> Component<CHILDWIDGET>
        where CHILDWIDGET: Widget + 'static,
              CHILDWIDGET::Msg: DisplayVariant + 'static,
              CHILDWIDGET::Root: IsA<gtk::Widget> + IsA<Object> + WidgetExt,
              WIDGET: Widget,
    {
        let (widget, component, child_relm) = create_widget::<CHILDWIDGET>(relm.context(), model_param);
        self.add(widget.widget());
        component.on_add(self.clone());
        init_component::<CHILDWIDGET>(widget.stream(), component, relm.context(), &child_relm);
        widget
    }

    fn remove_widget<WIDGET>(&self, component: Component<WIDGET>)
        where WIDGET: Widget,
              WIDGET::Root: IsA<gtk::Widget>,
    {
        self.remove(component.widget());
    }
}
