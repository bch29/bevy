use crate::{color::Color, pass::Operations, render_resource::TextureViewId};

#[derive(Debug, Clone)]
pub enum TextureAttachment {
    Id(TextureViewId),
    Input(String),
}

impl TextureAttachment {
    pub fn get_texture_view_id(&self) -> Option<TextureViewId> {
        if let TextureAttachment::Id(texture_id) = self {
            Some(*texture_id)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClearColor(pub Color);

impl Default for ClearColor {
    fn default() -> Self {
        Self(Color::rgb(0.4, 0.4, 0.4))
    }
}

#[derive(Debug, Clone)]
pub struct RenderPassColorAttachment {
    /// The actual color attachment.
    pub attachment: TextureAttachment,

    /// The resolve target for this color attachment, if any.
    pub resolve_target: Option<TextureAttachment>,

    /// What operations will be performed on this color attachment.
    pub ops: Operations<Color>,
}

#[derive(Debug, Clone)]
pub struct RenderPassDepthStencilAttachment {
    pub attachment: TextureAttachment,
    /// What operations will be performed on the depth part of the attachment.
    pub depth_ops: Option<Operations<f32>>,
    /// What operations will be performed on the stencil part of the attachment.
    pub stencil_ops: Option<Operations<u32>>,
}

// A set of pipeline bindings and draw calls with color and depth outputs
#[derive(Debug, Clone)]
pub struct PassDescriptor {
    pub color_attachments: Vec<RenderPassColorAttachment>,
    pub depth_stencil_attachment: Option<RenderPassDepthStencilAttachment>,
    pub sample_count: u32,
}
