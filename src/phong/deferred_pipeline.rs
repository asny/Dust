
use crate::math::*;
use crate::definition::*;
use crate::core::*;
use crate::camera::*;
use crate::light::*;
use crate::effect::*;

///
/// Used for debug purposes.
///
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DebugType {POSITION, NORMAL, COLOR, DEPTH, DIFFUSE, SPECULAR, POWER, NONE}

///
/// Deferred pipeline based on the Phong reflection model supporting a performance-limited
/// amount of directional, point and spot lights with shadows. Supports colored, textured and instanced meshes.
///
pub struct PhongDeferredPipeline {
    context: Context,
    ambient_light_effect: ImageEffect,
    directional_light_effect: ImageEffect,
    point_light_effect: ImageEffect,
    spot_light_effect: ImageEffect,
    debug_effect: Option<ImageEffect>,
    ///
    /// Set this to visualize the positions, normals etc. for debug purposes.
    ///
    pub debug_type: DebugType,
    geometry_pass_texture: Option<ColorTargetTexture2DArray>,
    geometry_pass_depth_texture: Option<DepthTargetTexture2DArray>
}

impl PhongDeferredPipeline
{
    ///
    /// Constructor.
    ///
    pub fn new(context: &Context) -> Result<Self, Error>
    {
        let renderer = Self {
            context: context.clone(),
            ambient_light_effect: ImageEffect::new(context, &format!("{}\n{}\n{}",
                                                                       &include_str!("shaders/light_shared.frag"),
                                                                       &include_str!("shaders/deferred_light_shared.frag"),
                                                                       &include_str!("shaders/ambient_light.frag")))?,
            directional_light_effect: ImageEffect::new(context, &format!("{}\n{}\n{}",
                                                                       &include_str!("shaders/light_shared.frag"),
                                                                       &include_str!("shaders/deferred_light_shared.frag"),
                                                                       &include_str!("shaders/directional_light.frag")))?,
            point_light_effect: ImageEffect::new(context, &format!("{}\n{}\n{}",
                                                                       &include_str!("shaders/light_shared.frag"),
                                                                       &include_str!("shaders/deferred_light_shared.frag"),
                                                                       &include_str!("shaders/point_light.frag")))?,
            spot_light_effect: ImageEffect::new(context, &format!("{}\n{}\n{}",
                                                                       &include_str!("shaders/light_shared.frag"),
                                                                       &include_str!("shaders/deferred_light_shared.frag"),
                                                                       &include_str!("shaders/spot_light.frag")))?,
            debug_effect: None,
            debug_type: DebugType::NONE,
            geometry_pass_texture: Some(ColorTargetTexture2DArray::new(context, 1, 1, 2,
                                                                       Interpolation::Nearest, Interpolation::Nearest, None, Wrapping::ClampToEdge,
                                                                       Wrapping::ClampToEdge, Format::RGBA8)?),
            geometry_pass_depth_texture: Some(DepthTargetTexture2DArray::new(context, 1, 1, 1, Wrapping::ClampToEdge,
                                                                             Wrapping::ClampToEdge, DepthFormat::Depth32F)?)
        };

        renderer.ambient_light_effect.program().use_texture(renderer.geometry_pass_texture(), "gbuffer")?;
        renderer.ambient_light_effect.program().use_texture(renderer.geometry_pass_depth_texture_array(), "depthMap")?;
        Ok(renderer)
    }

    ///
    /// Render the geometry and surface material parameters of Phong deferred [meshes](crate::PhongDeferredMesh)
    /// or [instanced meshes](crate::PhongDeferredInstancedMesh) by calling the *render_geometry* on
    /// either type of mesh inside the **render** closure.
    /// This function must not be called in a render target render function, but needs to be followed
    /// by a call to [light_pass](Self::light_pass) which must be inside a render target render function.
    ///
    pub fn geometry_pass<F: FnOnce() -> Result<(), Error>>(&mut self, width: usize, height: usize, render: F) -> Result<(), Error>
    {
        self.geometry_pass_texture = Some(ColorTargetTexture2DArray::new(&self.context, width, height, 2,
                                                                         Interpolation::Nearest, Interpolation::Nearest, None, Wrapping::ClampToEdge,
                                                                         Wrapping::ClampToEdge, Format::RGBA8)?);
        self.geometry_pass_depth_texture = Some(DepthTargetTexture2DArray::new(&self.context, width, height, 1, Wrapping::ClampToEdge,
                                                                               Wrapping::ClampToEdge, DepthFormat::Depth32F)?);
        RenderTargetArray::new(&self.context, self.geometry_pass_texture.as_ref().unwrap(), self.geometry_pass_depth_texture.as_ref().unwrap())?
            .write(&ClearState::default(), &[0, 1], 0, render)?;
        Ok(())
    }

    ///
    /// Uses the geometry and surface material parameters written in the last [geometry_pass](Self::geometry_pass) call
    /// and all of the given lights
    /// to shade the Phong deferred [meshes](crate::PhongDeferredMesh) or [instanced meshes](crate::PhongDeferredInstancedMesh).
    /// Must be called in a render target render function,
    /// for example in the callback function of [Screen::write](crate::Screen::write).
    ///
    pub fn light_pass(&mut self, viewport: Viewport, camera: &Camera, ambient_light: Option<&AmbientLight>, directional_lights: &[&DirectionalLight],
                      spot_lights: &[&SpotLight], point_lights: &[&PointLight]) -> Result<(), Error>
    {
        let mut render_states = RenderStates {cull: CullType::Back, depth_test: DepthTestType::LessOrEqual, ..Default::default()};

        if self.debug_type != DebugType::NONE {
            if self.debug_effect.is_none() {
                self.debug_effect = Some(ImageEffect::new(&self.context, include_str!("shaders/debug.frag")).unwrap());
            }
            self.debug_effect.as_ref().unwrap().program().use_uniform_mat4("viewProjectionInverse", &(camera.projection() * camera.view()).invert().unwrap())?;
            self.debug_effect.as_ref().unwrap().program().use_texture(self.geometry_pass_texture(), "gbuffer")?;
            self.debug_effect.as_ref().unwrap().program().use_texture(self.geometry_pass_depth_texture_array(), "depthMap")?;
            self.debug_effect.as_ref().unwrap().program().use_uniform_int("type", &(self.debug_type as i32))?;
            self.debug_effect.as_ref().unwrap().apply(render_states, viewport)?;
            return Ok(());
        }

        // Ambient light
        if let Some(light) = ambient_light {
            self.ambient_light_effect.program().use_texture(self.geometry_pass_texture(), "gbuffer")?;
            self.ambient_light_effect.program().use_texture(self.geometry_pass_depth_texture_array(), "depthMap")?;
            self.ambient_light_effect.program().use_uniform_vec3("ambientColor", &(light.color * light.intensity))?;
            self.ambient_light_effect.apply(render_states, viewport)?;
            render_states.blend = Some(BlendParameters::ADD);
        }

        // Directional light
        for light in directional_lights {
            self.directional_light_effect.program().use_texture(self.geometry_pass_texture(), "gbuffer")?;
            self.directional_light_effect.program().use_texture(self.geometry_pass_depth_texture_array(), "depthMap")?;
            self.directional_light_effect.program().use_uniform_vec3("eyePosition", &camera.position())?;
            self.directional_light_effect.program().use_uniform_mat4("viewProjectionInverse", &(camera.projection() * camera.view()).invert().unwrap())?;
            self.directional_light_effect.program().use_texture(light.shadow_map(), "shadowMap")?;
            self.directional_light_effect.program().use_uniform_block(light.buffer(), "DirectionalLightUniform");
            self.directional_light_effect.apply(render_states, viewport)?;
            render_states.blend = Some(BlendParameters::ADD);
        }

        // Spot lights
        for light in spot_lights {
            self.spot_light_effect.program().use_texture(self.geometry_pass_texture(), "gbuffer")?;
            self.spot_light_effect.program().use_texture(self.geometry_pass_depth_texture_array(), "depthMap")?;
            self.spot_light_effect.program().use_uniform_vec3("eyePosition", &camera.position())?;
            self.spot_light_effect.program().use_uniform_mat4("viewProjectionInverse", &(camera.projection() * camera.view()).invert().unwrap())?;
            self.spot_light_effect.program().use_texture(light.shadow_map(), "shadowMap")?;
            self.spot_light_effect.program().use_uniform_block(light.buffer(), "SpotLightUniform");
            self.spot_light_effect.apply(render_states, viewport)?;
            render_states.blend = Some(BlendParameters::ADD);
        }

        // Point lights
        for light in point_lights {
            self.point_light_effect.program().use_texture(self.geometry_pass_texture(), "gbuffer")?;
            self.point_light_effect.program().use_texture(self.geometry_pass_depth_texture_array(), "depthMap")?;
            self.point_light_effect.program().use_uniform_vec3("eyePosition", &camera.position())?;
            self.point_light_effect.program().use_uniform_mat4("viewProjectionInverse", &(camera.projection() * camera.view()).invert().unwrap())?;
            self.point_light_effect.program().use_uniform_block(light.buffer(), "PointLightUniform");
            self.point_light_effect.apply(render_states, viewport)?;
            render_states.blend = Some(BlendParameters::ADD);
        }

        Ok(())
    }

    pub fn geometry_pass_texture(&self) -> &dyn Texture
    {
        self.geometry_pass_texture.as_ref().unwrap()
    }
    pub fn geometry_pass_depth_texture_array(&self) -> &dyn Texture
    {
        self.geometry_pass_depth_texture.as_ref().unwrap()
    }

    pub fn geometry_pass_depth_texture(&self) -> DepthTargetTexture2D
    {
        let depth_array = self.geometry_pass_depth_texture.as_ref().unwrap();
        let depth_texture = DepthTargetTexture2D::new(&self.context, depth_array.width(), depth_array.height(),Wrapping::ClampToEdge,
                                           Wrapping::ClampToEdge, DepthFormat::Depth32F).unwrap();

        RenderTargetArray::new_depth(&self.context, depth_array).unwrap()
            .copy_depth(0, &RenderTarget::new_depth(&self.context, &depth_texture).unwrap(),
                        Viewport::new_at_origo(depth_array.width(), depth_array.height())).unwrap();
        depth_texture
    }
}