use std::{any::Any, marker::PhantomData};

use glam::Vec2;

use crate::{
    gpu::{
        context::{Context, ContextError},
        pass::Pass,
        text::TextRenderer,
    },
    scene::DrawContext,
    ui::widget::Widget,
};

pub struct WidgetContainer {
    tree: taffy::TaffyTree<Box<dyn Widget>>,
    root: taffy::NodeId,
}

#[derive(Clone, Copy)]
pub struct WidgetHandle<W> {
    id: taffy::NodeId,
    _w: PhantomData<fn() -> W>,
}

impl WidgetContainer {
    pub fn new() -> Result<WidgetContainer, taffy::TaffyError> {
        let mut tree = taffy::TaffyTree::new();

        let root = tree.new_leaf(taffy::Style::default()).unwrap();

        Ok(WidgetContainer { tree, root })
    }

    pub fn root(&self) -> taffy::NodeId {
        self.root
    }

    pub fn add<W: Widget>(
        &mut self,
        parent: taffy::NodeId,
        widget: W,
    ) -> Result<WidgetHandle<W>, taffy::TaffyError> {
        let style = widget.style();
        let child = self.tree.new_leaf_with_context(style, Box::new(widget))?;
        self.tree.add_child(parent, child)?;
        Ok(WidgetHandle {
            id: child,
            _w: PhantomData,
        })
    }

    pub fn get<W: Widget>(&mut self, h: WidgetHandle<W>) -> &mut W {
        let w: &mut dyn Any = &mut **self.tree.get_node_context_mut(h.id).unwrap();
        w.downcast_mut().unwrap()
    }

    pub fn compute(
        &mut self,
        viewport_physical: Vec2,
        scale: f32,
        text: &mut TextRenderer,
    ) -> Result<(), taffy::TaffyError> {
        let mut style = self.tree.style(self.root)?.clone();
        style.size = taffy::Size {
            width: taffy::Dimension::length(viewport_physical.x / scale),
            height: taffy::Dimension::length(viewport_physical.y / scale),
        };
        self.tree.set_style(self.root, style)?;

        self.tree.compute_layout_with_measure(
            self.root,
            taffy::Size::max_content(),
            |inputs, _node_id, node_context, style| {
                taffy::compute_leaf_layout(
                    inputs,
                    style,
                    |_, _| 0.,
                    |known, available| match node_context {
                        Some(widget) => widget.measure(known, available, text),
                        None => taffy::Size::zero(),
                    },
                )
            },
        )?;

        Ok(())
    }

    pub fn draw(
        &mut self,
        gpu: &Context<'_>,
        draw: &mut DrawContext,
        pass: &mut Pass<'_, '_, '_>,
    ) -> Result<(), ContextError> {
        self.compute(
            Vec2 {
                x: pass.ctx.config.width as f32,
                y: pass.ctx.config.height as f32,
            },
            draw.scale,
            &mut draw.text,
        )?;

        let items = {
            let mut out = Vec::new();
            self.layout(self.root, Vec2::ZERO, &mut out);
            out
        };

        for (node, rect) in items {
            if let Some(widget) = self.tree.get_node_context_mut(node) {
                let physical_rect = taffy::Rect {
                    left: (rect.left * draw.scale).round(),
                    right: (rect.right * draw.scale).round(),
                    top: (rect.top * draw.scale).round(),
                    bottom: (rect.bottom * draw.scale).round(),
                };
                widget.draw(physical_rect, gpu, draw, pass)?;
            }
        }

        Ok(())
    }

    fn layout(
        &self,
        node: taffy::NodeId,
        origin: Vec2,
        out: &mut Vec<(taffy::NodeId, taffy::Rect<f32>)>,
    ) {
        let layout = self.tree.layout(node).expect("called compute before this");
        let position = origin + Vec2::new(layout.location.x, layout.location.y);

        out.push((
            node,
            taffy::Rect {
                left: position.x,
                right: position.x + layout.size.width,
                top: position.y,
                bottom: position.y + layout.size.height,
            },
        ));

        for child in self.tree.children(node).expect("node is valid") {
            self.layout(child, position, out);
        }
    }
}
