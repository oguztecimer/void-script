use std::sync::Arc;
use tiny_skia::Pixmap;
use winit::window::Window;

#[cfg(not(target_os = "macos"))]
use softbuffer::Surface;

use crate::unit::UnitManager;

/// Handles blitting to the display surface.
///
/// On macOS: renders directly to a CALayer using a CGBitmapContext with premultiplied
/// alpha, bypassing softbuffer's opaque CGImage format (`NoneSkipFirst`).
///
/// On other platforms: blits via softbuffer using the `0x00RRGGBB` pixel format.
pub struct Renderer {
    /// The canvas Pixmap that units draw into each frame.
    canvas: Pixmap,
    /// Global integer pixel scale applied to all rendering.
    pub pixel_scale: u32,

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

        Self {
            canvas,
            pixel_scale: 1,
            #[cfg(target_os = "macos")]
            pixel_buf: vec![0u8; (strip_width * strip_height * 4) as usize],
            #[cfg(target_os = "macos")]
            layer: None,
        }
    }

    /// Resize the canvas and macOS CALayer frame to new dimensions.
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
                let scale = macos_layer.layer().contentsScale();
                macos_layer.layer().setContentsScale(scale);
                CATransaction::commit();
            }
        }
    }

    /// Check whether the pixel at (x, y) in logical coordinates is non-transparent.
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

    // -----------------------------------------------------------------------
    // Render paths
    // -----------------------------------------------------------------------

    /// Non-macOS render path: blit via softbuffer using `0x00RRGGBB` pixel format.
    #[cfg(not(target_os = "macos"))]
    pub fn render(
        &mut self,
        surface: &mut Surface<Arc<Window>, Arc<Window>>,
        _width: u32,
        height: u32,
        units: &UnitManager,
        dock_height: u32,
    ) {
        // Clear canvas to fully transparent.
        self.canvas.data_mut().fill(0);

        // Draw all units.
        units.draw_all(&mut self.canvas, height, self.pixel_scale, dock_height);

        // Blit canvas pixels to softbuffer.
        let mut buffer = surface.buffer_mut().expect("Failed to get surface buffer");
        for (dst, src) in buffer.iter_mut().zip(self.canvas.pixels()) {
            let a = src.alpha();
            if a == 0 {
                *dst = 0u32;
            } else {
                let r = (src.red() as u32 * 255 / a as u32).min(255);
                let g = (src.green() as u32 * 255 / a as u32).min(255);
                let b = (src.blue() as u32 * 255 / a as u32).min(255);
                *dst = ((a as u32) << 24) | (r << 16) | (g << 8) | b;
            }
        }
        buffer.present().expect("Failed to present surface buffer");
    }

    /// macOS render path: draw via a `CGBitmapContext` with premultiplied alpha.
    #[cfg(target_os = "macos")]
    pub fn render(
        &mut self,
        _surface: &mut softbuffer::Surface<Arc<Window>, Arc<Window>>,
        width: u32,
        height: u32,
        units: &UnitManager,
        dock_height: u32,
    ) {
        use objc2_core_graphics::{
            CGImage, CGImageAlphaInfo, CGImageByteOrderInfo,
            CGColorSpace,
            CGBitmapContextCreate, CGBitmapContextCreateImage,
        };
        use objc2_quartz_core::CATransaction;

        if self.layer.is_none() {
            return;
        }

        // Clear canvas to fully transparent.
        self.canvas.data_mut().fill(0);

        // Draw all units.
        units.draw_all(&mut self.canvas, height, self.pixel_scale, dock_height);

        let macos_layer = self.layer.as_ref().unwrap();

        // Convert tiny_skia premultiplied RGBA → CoreGraphics premultiplied BGRA.
        let w = width as usize;
        let h = height as usize;
        for (i, px) in self.canvas.pixels().iter().enumerate() {
            let base = i * 4;
            self.pixel_buf[base]     = px.blue();
            self.pixel_buf[base + 1] = px.green();
            self.pixel_buf[base + 2] = px.red();
            self.pixel_buf[base + 3] = px.alpha();
        }

        let bitmap_info: u32 = CGImageAlphaInfo::PremultipliedFirst.0
            | CGImageByteOrderInfo::Order32Little.0;

        let color_space = match CGColorSpace::new_device_rgb() {
            Some(cs) => cs,
            None => return,
        };

        let ctx = unsafe {
            CGBitmapContextCreate(
                std::ptr::null_mut(),
                w,
                h,
                8,
                w * 4,
                Some(&color_space),
                bitmap_info,
            )
        };
        let ctx = match ctx {
            Some(c) => c,
            None => return,
        };

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

        let image: objc2_core_foundation::CFRetained<CGImage> =
            match CGBitmapContextCreateImage(Some(&ctx)) {
                Some(img) => img,
                None => return,
            };

        let cg_as_anyobj: &objc2::runtime::AnyObject = unsafe {
            &*(&*image as *const CGImage as *const objc2::runtime::AnyObject)
        };
        CATransaction::begin();
        CATransaction::setDisableActions(true);
        unsafe {
            macos_layer.layer().setContents(Some(cg_as_anyobj));
        }
        CATransaction::commit();
    }

    /// macOS: create a dedicated CALayer for rendering and add it to the NSView hierarchy.
    #[cfg(target_os = "macos")]
    pub fn set_window(&mut self, window: &Arc<Window>) {
        use objc2::runtime::AnyObject;
        use objc2::msg_send;
        use objc2_foundation::MainThreadMarker;
        use objc2_quartz_core::CALayer;
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

        let _mtm = MainThreadMarker::new()
            .expect("set_window must be called from the main thread");

        let handle = window.window_handle().expect("Failed to get window handle");
        let ns_view: &AnyObject = match handle.as_raw() {
            RawWindowHandle::AppKit(h) => unsafe { h.ns_view.cast().as_ref() },
            _ => panic!("Expected AppKit window handle on macOS"),
        };

        let root_layer: objc2::rc::Retained<CALayer> = unsafe {
            msg_send![ns_view, layer]
        };

        root_layer.setOpaque(false);
        root_layer.setBackgroundColor(None);

        let sb_sublayers: objc2::rc::Retained<objc2_foundation::NSArray<CALayer>> = unsafe {
            msg_send![&*root_layer, sublayers]
        };
        for i in 0..sb_sublayers.len() {
            let sublayer = sb_sublayers.objectAtIndex(i);
            sublayer.setOpaque(false);
            sublayer.setBackgroundColor(None);
        }

        let our_layer = CALayer::new();
        our_layer.setOpaque(false);
        our_layer.setGeometryFlipped(true);

        // Pixel-perfect rendering — disable bilinear filtering.
        unsafe {
            our_layer.setMagnificationFilter(objc2_quartz_core::kCAFilterNearest);
            our_layer.setMinificationFilter(objc2_quartz_core::kCAFilterNearest);
        }

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
