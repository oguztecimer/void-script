use std::sync::Arc;
use tiny_skia::Pixmap;
use winit::window::Window;

#[cfg(not(target_os = "macos"))]
use softbuffer::Surface;

use crate::animation::AnimationPlayer;

/// Handles blitting to the display surface.
///
/// On macOS: renders directly to a CALayer using a CGBitmapContext with premultiplied
/// alpha, bypassing softbuffer's opaque CGImage format (`NoneSkipFirst`).
///
/// On other platforms: blits via softbuffer using the `0x00RRGGBB` pixel format.
pub struct Renderer {
    /// The canvas Pixmap that the AnimationPlayer draws into each frame.
    canvas: Pixmap,
    /// Horizontal position of the dog sprite within the strip.
    dog_x: i32,

    /// macOS: pixel buffer (BGRA premultiplied) for building alpha-aware CGImages.
    #[cfg(target_os = "macos")]
    pixel_buf: Vec<u8>,
    /// macOS: the CALayer we own and render into.
    #[cfg(target_os = "macos")]
    layer: Option<MacosLayer>,
}

/// Wrapper holding the CALayer used for macOS rendering.
///
/// We create a dedicated CALayer (not softbuffer's internal one) and add it
/// directly to the NSView's root layer. This gives us full control over the
/// bitmap format used when setting `contents`, allowing premultiplied alpha.
#[cfg(target_os = "macos")]
struct MacosLayer {
    layer: objc2::rc::Retained<objc2_quartz_core::CALayer>,
}

#[cfg(target_os = "macos")]
impl MacosLayer {
    fn layer(&self) -> &objc2_quartz_core::CALayer {
        &self.layer
    }
}

// SAFETY: our app uses CALayer only from the main thread — winit guarantees this.
#[cfg(target_os = "macos")]
unsafe impl Send for MacosLayer {}
#[cfg(target_os = "macos")]
unsafe impl Sync for MacosLayer {}

impl Renderer {
    pub fn new(strip_width: u32, strip_height: u32) -> Self {
        let canvas =
            Pixmap::new(strip_width, strip_height).expect("Failed to create canvas Pixmap");

        // Start the dog near the left quarter of the strip so it is always
        // visible and has room to walk rightward from the very first movement.
        // Random placement was removed because it could spawn the dog off to one
        // side where the user cannot see the animations clearly during the demo.
        let dog_x = if strip_width > 48 {
            (strip_width / 4) as i32
        } else {
            0
        };

        Self {
            canvas,
            dog_x,
            #[cfg(target_os = "macos")]
            pixel_buf: vec![0u8; (strip_width * strip_height * 4) as usize],
            #[cfg(target_os = "macos")]
            layer: None,
        }
    }

    /// Resize the canvas and macOS CALayer frame to new dimensions.
    ///
    /// Called on OS-initiated window resize events. Per RESEARCH.md Pitfall 3, the CALayer
    /// frame MUST be updated here or rendering appears squished on macOS.
    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        self.canvas = Pixmap::new(new_width, new_height)
            .expect("Failed to resize canvas Pixmap");

        #[cfg(target_os = "macos")]
        {
            self.pixel_buf = vec![0u8; (new_width * new_height * 4) as usize];
            if let Some(macos_layer) = &self.layer {
                use objc2_core_foundation::{CGPoint, CGRect, CGSize};
                use objc2_quartz_core::CATransaction;
                CATransaction::begin();
                CATransaction::setDisableActions(true);
                let new_bounds = CGRect {
                    origin: CGPoint { x: 0.0, y: 0.0 },
                    size: CGSize {
                        width: new_width as f64,
                        height: new_height as f64,
                    },
                };
                macos_layer.layer().setFrame(new_bounds);
                // Keep the contentsScale in sync with the window.
                let scale = macos_layer.layer().contentsScale();
                macos_layer.layer().setContentsScale(scale);
                CATransaction::commit();
            }
        }
    }

    /// Check whether the pixel at (x, y) in logical coordinates is non-transparent.
    /// Returns true if the pixel has alpha > 0 (i.e., the dog sprite is there).
    pub fn hit_test_at(&self, x: f64, y: f64) -> bool {
        let ix = x as u32;
        let iy = y as u32;
        let w = self.canvas.width();
        let h = self.canvas.height();
        if ix >= w || iy >= h {
            return false;
        }
        let idx = (iy * w + ix) as usize;
        if let Some(px) = self.canvas.pixels().get(idx) {
            px.alpha() > 0
        } else {
            false
        }
    }

    /// Return current horizontal position of the dog.
    pub fn dog_x(&self) -> i32 {
        self.dog_x
    }

    /// Set horizontal position of the dog.
    pub fn set_dog_x(&mut self, x: i32) {
        self.dog_x = x;
    }

    // -----------------------------------------------------------------------
    // Fetch-specific drawing helpers
    // -----------------------------------------------------------------------

    /// Draw the ball as a filled orange circle at (bx, by).
    fn draw_ball(&mut self, bx: f32, by: f32) {
        const BALL_RADIUS: f32 = 6.0;
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(240, 140, 30, 255); // Orange

        if let Some(path) = tiny_skia::PathBuilder::from_circle(bx, by, BALL_RADIUS) {
            self.canvas.fill_path(
                &path,
                &paint,
                tiny_skia::FillRule::Winding,
                tiny_skia::Transform::identity(),
                None,
            );
        }
    }

    /// Draw per-particle colored squares. Used for celebration sparkles.
    fn draw_colored_particles(&mut self, particles: &[(f32, f32, f32, u8, u8, u8)]) {
        for &(x, y, alpha, r, g, b) in particles {
            let a = (alpha * 255.0) as u8;
            if a == 0 {
                continue;
            }
            let mut paint = tiny_skia::Paint::default();
            paint.set_color_rgba8(r, g, b, a);
            if let Some(rect) = tiny_skia::Rect::from_xywh(x, y, 6.0, 6.0) {
                self.canvas
                    .fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Render paths
    // -----------------------------------------------------------------------

    /// Non-macOS render path: blit via softbuffer using `0x00RRGGBB` pixel format.
    ///
    /// softbuffer on Windows/Linux uses `NoneSkipFirst | Order32Little`, which means
    /// the window's overall opacity (set by `with_transparent(true)`) provides the
    /// transparency — per-pixel alpha is not needed on these platforms.
    #[cfg(not(target_os = "macos"))]
    pub fn render(
        &mut self,
        surface: &mut Surface<Arc<Window>, Arc<Window>>,
        _width: u32,
        height: u32,
        player: &AnimationPlayer,
        particles: &[(f32, f32, f32)],
        overlay: Option<(i32, i32, u32, u32, u8, u8, u8)>,
        ball: Option<(f32, f32)>,
        colored_particles: &[(f32, f32, f32, u8, u8, u8)],
    ) {
        // Clear canvas to fully transparent.
        self.canvas.data_mut().fill(0);

        // Draw current sprite frame at the bottom of the strip.
        let y = height as i32 - player.frame_height() as i32;
        player.draw(&mut self.canvas, self.dog_x, y);

        // Draw visual overlay (food bowl / soap bubbles) if any.
        if let Some((ox, oy, ow, oh, r, g, b)) = overlay {
            self.draw_overlay(ox, oy, ow, oh, r, g, b, 200);
        }

        // Draw ball if active.
        if let Some((bx, by)) = ball {
            self.draw_ball(bx, by);
        }

        // Draw heart particles on top.
        self.draw_particles(particles);

        // Draw colored particles (celebration sparkles).
        self.draw_colored_particles(colored_particles);

        // Blit canvas pixels to softbuffer.
        let mut buffer = surface.buffer_mut().expect("Failed to get surface buffer");
        for (dst, src) in buffer.iter_mut().zip(self.canvas.pixels()) {
            let a = src.alpha();
            if a == 0 {
                *dst = 0u32;
            } else {
                // Demultiply premultiplied RGBA → straight RGB, pack as 0x00RRGGBB.
                let r = (src.red() as u32 * 255 / a as u32).min(255);
                let g = (src.green() as u32 * 255 / a as u32).min(255);
                let b = (src.blue() as u32 * 255 / a as u32).min(255);
                *dst = (r << 16) | (g << 8) | b;
            }
        }
        buffer.present().expect("Failed to present surface buffer");
    }

    /// macOS render path: draw via a `CGBitmapContext` with premultiplied alpha.
    ///
    /// softbuffer's macOS backend hardcodes `CGImageAlphaInfo::NoneSkipFirst` when
    /// creating the `CGImage` it uses as the CALayer's `contents`. This tells Core
    /// Graphics to ignore the alpha channel — every pixel is rendered fully opaque,
    /// causing the transparent areas to appear as solid black.
    ///
    /// We bypass softbuffer's buffer entirely and write our own `CGImage` (with
    /// `PremultipliedFirst | Order32Little`, i.e. BGRA premultiplied) to a separate
    /// CALayer that we own.  The `_surface` parameter is unused here; softbuffer's
    /// surface is still needed to keep the NSView layer-backed (it calls
    /// `setWantsLayer:YES` in `Surface::new`).
    #[cfg(target_os = "macos")]
    pub fn render(
        &mut self,
        _surface: &mut softbuffer::Surface<Arc<Window>, Arc<Window>>,
        width: u32,
        height: u32,
        player: &AnimationPlayer,
        particles: &[(f32, f32, f32)],
        overlay: Option<(i32, i32, u32, u32, u8, u8, u8)>,
        ball: Option<(f32, f32)>,
        colored_particles: &[(f32, f32, f32, u8, u8, u8)],
    ) {
        use objc2_core_graphics::{
            CGImage, CGImageAlphaInfo, CGImageByteOrderInfo,
            CGColorSpace,
            CGBitmapContextCreate, CGBitmapContextCreateImage,
        };
        use objc2_quartz_core::CATransaction;

        // Guard: layer must be configured before we can render.
        if self.layer.is_none() {
            return;
        }

        // Clear canvas to fully transparent.
        self.canvas.data_mut().fill(0);

        // Draw current sprite frame at the bottom of the strip.
        let y = height as i32 - player.frame_height() as i32;
        player.draw(&mut self.canvas, self.dog_x, y);

        // Draw visual overlay (food bowl / soap bubbles) if any.
        if let Some((ox, oy, ow, oh, r, g, b)) = overlay {
            self.draw_overlay(ox, oy, ow, oh, r, g, b, 200);
        }

        // Draw ball if active.
        if let Some((bx, by)) = ball {
            self.draw_ball(bx, by);
        }

        // Draw heart particles on top.
        self.draw_particles(particles);

        // Draw colored particles (celebration sparkles).
        self.draw_colored_particles(colored_particles);

        // Borrow macos_layer only after all mutable canvas operations are done.
        let macos_layer = self.layer.as_ref().unwrap();

        // Convert tiny_skia premultiplied RGBA → CoreGraphics premultiplied BGRA.
        //
        // tiny_skia pixels in memory: [R, G, B, A] (RGBA, premultiplied).
        // CoreGraphics with PremultipliedFirst|Order32Little expects: [B, G, R, A] (BGRA).
        let w = width as usize;
        let h = height as usize;
        for (i, px) in self.canvas.pixels().iter().enumerate() {
            let base = i * 4;
            self.pixel_buf[base]     = px.blue();
            self.pixel_buf[base + 1] = px.green();
            self.pixel_buf[base + 2] = px.red();
            self.pixel_buf[base + 3] = px.alpha();
        }

        // kCGImageAlphaPremultipliedFirst | kCGBitmapByteOrder32Little = BGRA premultiplied.
        let bitmap_info: u32 = CGImageAlphaInfo::PremultipliedFirst.0
            | CGImageByteOrderInfo::Order32Little.0;

        let color_space = match CGColorSpace::new_device_rgb() {
            Some(cs) => cs,
            None => return,
        };

        // Create a CGBitmapContext with CG-managed memory (data = null → CG allocates).
        // We then write our pixels directly into CG's buffer via CGBitmapContextGetData.
        // This avoids any lifetime issues with our pixel_buf being referenced by the CGImage.
        //
        // SAFETY: null data pointer is a documented valid argument — CG allocates its own buffer.
        let ctx = unsafe {
            CGBitmapContextCreate(
                std::ptr::null_mut(),
                w,
                h,
                8,       // bits per component
                w * 4,   // bytes per row
                Some(&color_space),
                bitmap_info,
            )
        };
        let ctx = match ctx {
            Some(c) => c,
            None => return,
        };

        // Copy our BGRA pixel data into the CG-owned buffer.
        // SAFETY: CGBitmapContextGetData returns a valid pointer to `w * h * 4` bytes.
        use objc2_core_graphics::CGBitmapContextGetData;
        let ctx_data_ptr = CGBitmapContextGetData(Some(&ctx));
        if !ctx_data_ptr.is_null() {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    self.pixel_buf.as_ptr(),
                    ctx_data_ptr as *mut u8,
                    w * h * 4,
                );
            }
        }

        // Create a CGImage snapshot from the context (copies the pixel data at this point).
        // The explicit type annotation resolves the ambiguous `.cast()` call below.
        let image: objc2_core_foundation::CFRetained<CGImage> =
            match CGBitmapContextCreateImage(Some(&ctx)) {
                Some(img) => img,
                None => return,
            };

        // Set the image as CALayer contents inside a no-animation transaction.
        // CGImage is toll-free bridged: cast *const CGImage → *const AnyObject is valid.
        let cg_as_anyobj: &objc2::runtime::AnyObject = unsafe {
            &*(&*image as *const CGImage as *const objc2::runtime::AnyObject)
        };
        CATransaction::begin();
        CATransaction::setDisableActions(true);
        // SAFETY: CGImage is a valid `contents` type for CALayer (documented by Apple).
        unsafe {
            macos_layer.layer().setContents(Some(cg_as_anyobj));
        }
        CATransaction::commit();
    }

    // -----------------------------------------------------------------------
    // Particle and overlay helpers
    // -----------------------------------------------------------------------

    /// Draw heart particles as small 6×6 pink-red rectangles on the canvas.
    ///
    /// `particles` is a slice of `(x, y, alpha)` tuples. Alpha 0.0 = fully transparent,
    /// 1.0 = fully opaque. Particles with alpha == 0 are skipped.
    fn draw_particles(&mut self, particles: &[(f32, f32, f32)]) {
        for &(x, y, alpha) in particles {
            let a = (alpha * 255.0) as u8;
            if a == 0 {
                continue;
            }
            let mut paint = tiny_skia::Paint::default();
            paint.set_color_rgba8(255, 100, 180, a);
            if let Some(rect) = tiny_skia::Rect::from_xywh(x, y, 6.0, 6.0) {
                self.canvas.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
            }
        }
    }

    /// Draw a simple colored rectangle overlay near the dog (food bowl / soap bubbles).
    ///
    /// This is placeholder pixel art — a small colored block rendered at (x, y) with the
    /// given dimensions and color. Used for Phase 3 visual feedback.
    fn draw_overlay(&mut self, x: i32, y: i32, width: u32, height: u32, r: u8, g: u8, b: u8, alpha: u8) {
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(r, g, b, alpha);
        if let Some(rect) = tiny_skia::Rect::from_xywh(x as f32, y as f32, width as f32, height as f32) {
            self.canvas.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
        }
    }

    /// macOS: create a dedicated CALayer for rendering and add it to the NSView hierarchy.
    ///
    /// Must be called after `Surface::new()` (which calls `setWantsLayer:YES` on the NSView).
    /// We create our own sublayer with `isOpaque = false`, giving us full control over alpha.
    ///
    /// The `_surface` parameter is not used directly; it exists to document the ordering
    /// requirement (surface must be created first so the NSView is layer-backed).
    #[cfg(target_os = "macos")]
    pub fn set_window(&mut self, window: &Arc<Window>) {
        use objc2::runtime::AnyObject;
        use objc2::msg_send;
        use objc2_foundation::MainThreadMarker;
        use objc2_quartz_core::CALayer;
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

        // MainThreadMarker panics if not on the main thread — safe because winit
        // only calls `resumed()` (our caller) on the main thread.
        let _mtm = MainThreadMarker::new()
            .expect("set_window must be called from the main thread");

        let handle = window.window_handle().expect("Failed to get window handle");
        let ns_view: &AnyObject = match handle.as_raw() {
            RawWindowHandle::AppKit(h) => unsafe { h.ns_view.cast().as_ref() },
            _ => panic!("Expected AppKit window handle on macOS"),
        };

        // Get the NSView's root CALayer (softbuffer already called setWantsLayer:YES).
        let root_layer: objc2::rc::Retained<CALayer> = unsafe {
            msg_send![ns_view, layer]
        };

        // Make the root layer non-opaque so the NSWindow background (clearColor) shows through.
        root_layer.setOpaque(false);
        root_layer.setBackgroundColor(None);

        // Also make softbuffer's internal sublayer non-opaque.
        // softbuffer adds exactly one sublayer (CGImpl::new creates `let layer = CALayer::new()`).
        let sb_sublayers: objc2::rc::Retained<objc2_foundation::NSArray<CALayer>> = unsafe {
            msg_send![&*root_layer, sublayers]
        };
        for i in 0..sb_sublayers.len() {
            let sublayer = sb_sublayers.objectAtIndex(i);
            sublayer.setOpaque(false);
            sublayer.setBackgroundColor(None);
        }

        // Create our own CALayer that we control for alpha-enabled rendering.
        // We add it above softbuffer's layer so our pixels appear on top.
        let our_layer = CALayer::new();
        our_layer.setOpaque(false);
        our_layer.setGeometryFlipped(true); // Match CoreGraphics top-left origin.

        // Match the root layer's size and scale.
        let bounds = root_layer.bounds();
        our_layer.setFrame(bounds);
        let scale = root_layer.contentsScale();
        our_layer.setContentsScale(scale);

        root_layer.addSublayer(&our_layer);

        self.layer = Some(MacosLayer { layer: our_layer });
    }

    /// Non-macOS stub: no-op (softbuffer manages the surface directly).
    #[cfg(not(target_os = "macos"))]
    pub fn set_window(&mut self, _window: &Arc<Window>) {}
}
