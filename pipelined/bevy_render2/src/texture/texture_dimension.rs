// NOTE: These are currently just copies of the wgpu types, but they might change in the future

use bevy_math::Vec3;

/// Dimensions of a particular texture view.
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum TextureViewDimension {
    D1,
    D2,
    D2Array,
    Cube,
    CubeArray,
    D3,
}

/// Dimensionality of a texture.
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum TextureDimension {
    D1,
    D2,
    D3,
}

// TODO: use math type here
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Extent3d {
    pub width: u32,
    pub height: u32,
    pub depth_or_array_layers: u32,
}

impl Extent3d {
    pub fn new(width: u32, height: u32, depth_or_array_layers: u32) -> Self {
        Self {
            width,
            height,
            depth_or_array_layers,
        }
    }

    pub fn volume(&self) -> usize {
        (self.width * self.height * self.depth_or_array_layers) as usize
    }

    pub fn as_vec3(&self) -> Vec3 {
        Vec3::new(
            self.width as f32,
            self.height as f32,
            self.depth_or_array_layers as f32,
        )
    }
}

/// Type of data shaders will read from a texture.
#[derive(Copy, Hash, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum TextureSampleType {
    /// Sampling returns floats.
    ///
    /// If `filterable` is false, the texture can't be sampled with
    /// a filtering sampler.
    ///
    /// Example GLSL syntax:
    /// ```cpp,ignore
    /// layout(binding = 0)
    /// uniform texture2D t;
    /// ```
    Float { filterable: bool },
    /// Sampling does the depth reference comparison.
    ///
    /// Example GLSL syntax:
    /// ```cpp,ignore
    /// layout(binding = 0)
    /// uniform texture2DShadow t;
    /// ```
    Depth,
    /// Sampling returns signed integers.
    ///
    /// Example GLSL syntax:
    /// ```cpp,ignore
    /// layout(binding = 0)
    /// uniform itexture2D t;
    /// ```
    Sint,
    /// Sampling returns unsigned integers.
    ///
    /// Example GLSL syntax:
    /// ```cpp,ignore
    /// layout(binding = 0)
    /// uniform utexture2D t;
    /// ```
    Uint,
}

pub struct PixelInfo {
    pub type_size: usize,
    pub num_components: usize,
}

/// Underlying texture data format.
///
/// If there is a conversion in the format (such as srgb -> linear), The conversion listed is for
/// loading from texture in a shader. When writing to the texture, the opposite conversion takes
/// place.
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum TextureFormat {
    // Normal 8 bit formats
    R8Unorm = 0,
    R8Snorm = 1,
    R8Uint = 2,
    R8Sint = 3,

    // Normal 16 bit formats
    R16Uint = 4,
    R16Sint = 5,
    R16Float = 6,
    Rg8Unorm = 7,
    Rg8Snorm = 8,
    Rg8Uint = 9,
    Rg8Sint = 10,

    // Normal 32 bit formats
    R32Uint = 11,
    R32Sint = 12,
    R32Float = 13,
    Rg16Uint = 14,
    Rg16Sint = 15,
    Rg16Float = 16,
    Rgba8Unorm = 17,
    Rgba8UnormSrgb = 18,
    Rgba8Snorm = 19,
    Rgba8Uint = 20,
    Rgba8Sint = 21,
    Bgra8Unorm = 22,
    Bgra8UnormSrgb = 23,

    // Packed 32 bit formats
    Rgb10a2Unorm = 24,
    Rg11b10Float = 25,

    // Normal 64 bit formats
    Rg32Uint = 26,
    Rg32Sint = 27,
    Rg32Float = 28,
    Rgba16Uint = 29,
    Rgba16Sint = 30,
    Rgba16Float = 31,

    // Normal 128 bit formats
    Rgba32Uint = 32,
    Rgba32Sint = 33,
    Rgba32Float = 34,

    // Depth and stencil formats
    Depth32Float = 35,
    Depth24Plus = 36,
    Depth24PlusStencil8 = 37,
}

impl TextureFormat {
    pub fn pixel_info(&self) -> PixelInfo {
        let type_size = match self {
            // 8bit
            TextureFormat::R8Unorm
            | TextureFormat::R8Snorm
            | TextureFormat::R8Uint
            | TextureFormat::R8Sint
            | TextureFormat::Rg8Unorm
            | TextureFormat::Rg8Snorm
            | TextureFormat::Rg8Uint
            | TextureFormat::Rg8Sint
            | TextureFormat::Rgba8Unorm
            | TextureFormat::Rgba8UnormSrgb
            | TextureFormat::Rgba8Snorm
            | TextureFormat::Rgba8Uint
            | TextureFormat::Rgba8Sint
            | TextureFormat::Bgra8Unorm
            | TextureFormat::Bgra8UnormSrgb => 1,

            // 16bit
            TextureFormat::R16Uint
            | TextureFormat::R16Sint
            | TextureFormat::R16Float
            | TextureFormat::Rg16Uint
            | TextureFormat::Rg16Sint
            | TextureFormat::Rg16Float
            | TextureFormat::Rgba16Uint
            | TextureFormat::Rgba16Sint
            | TextureFormat::Rgba16Float => 2,

            // 32bit
            TextureFormat::R32Uint
            | TextureFormat::R32Sint
            | TextureFormat::R32Float
            | TextureFormat::Rg32Uint
            | TextureFormat::Rg32Sint
            | TextureFormat::Rg32Float
            | TextureFormat::Rgba32Uint
            | TextureFormat::Rgba32Sint
            | TextureFormat::Rgba32Float
            | TextureFormat::Depth32Float => 4,

            // special cases
            TextureFormat::Rgb10a2Unorm => 4,
            TextureFormat::Rg11b10Float => 4,
            TextureFormat::Depth24Plus => 3, // FIXME is this correct?
            TextureFormat::Depth24PlusStencil8 => 4,
        };

        let components = match self {
            TextureFormat::R8Unorm
            | TextureFormat::R8Snorm
            | TextureFormat::R8Uint
            | TextureFormat::R8Sint
            | TextureFormat::R16Uint
            | TextureFormat::R16Sint
            | TextureFormat::R16Float
            | TextureFormat::R32Uint
            | TextureFormat::R32Sint
            | TextureFormat::R32Float => 1,

            TextureFormat::Rg8Unorm
            | TextureFormat::Rg8Snorm
            | TextureFormat::Rg8Uint
            | TextureFormat::Rg8Sint
            | TextureFormat::Rg16Uint
            | TextureFormat::Rg16Sint
            | TextureFormat::Rg16Float
            | TextureFormat::Rg32Uint
            | TextureFormat::Rg32Sint
            | TextureFormat::Rg32Float => 2,

            TextureFormat::Rgba8Unorm
            | TextureFormat::Rgba8UnormSrgb
            | TextureFormat::Rgba8Snorm
            | TextureFormat::Rgba8Uint
            | TextureFormat::Rgba8Sint
            | TextureFormat::Bgra8Unorm
            | TextureFormat::Bgra8UnormSrgb
            | TextureFormat::Rgba16Uint
            | TextureFormat::Rgba16Sint
            | TextureFormat::Rgba16Float
            | TextureFormat::Rgba32Uint
            | TextureFormat::Rgba32Sint
            | TextureFormat::Rgba32Float => 4,

            // special cases
            TextureFormat::Rgb10a2Unorm
            | TextureFormat::Rg11b10Float
            | TextureFormat::Depth32Float
            | TextureFormat::Depth24Plus
            | TextureFormat::Depth24PlusStencil8 => 1,
        };

        PixelInfo {
            type_size,
            num_components: components,
        }
    }

    pub fn pixel_size(&self) -> usize {
        let info = self.pixel_info();
        info.type_size * info.num_components
    }
}

impl Default for TextureFormat {
    fn default() -> Self {
        if cfg!(target_os = "android") {
            // Bgra8UnormSrgb texture missing on some Android devices
            TextureFormat::Rgba8UnormSrgb
        } else {
            TextureFormat::Bgra8UnormSrgb
        }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    pub struct TextureUsage: u32 {
        /// Allows a texture to be the source in a [`CommandEncoder::copy_texture_to_buffer`] or
        /// [`CommandEncoder::copy_texture_to_texture`] operation.
        const COPY_SRC = 1;
        /// Allows a texture to be the destination in a  [`CommandEncoder::copy_texture_to_buffer`],
        /// [`CommandEncoder::copy_texture_to_texture`], or [`Queue::write_texture`] operation.
        const COPY_DST = 2;
        /// Allows a texture to be a [`BindingType::Texture`] in a bind group.
        const SAMPLED = 4;
        /// Allows a texture to be a [`BindingType::StorageTexture`] in a bind group.
        const STORAGE = 8;
        /// Allows a texture to be an output attachment of a renderpass.
        const RENDER_ATTACHMENT = 16;
    }
}
