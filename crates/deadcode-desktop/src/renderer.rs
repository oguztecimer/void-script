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
    /// Window reference for platform-specific rendering.
    window: Option<Arc<Window>>,

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
            window: None,
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
        if x < 0.0 || y < 0.0 {
            return false;
        }
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

    /// Windows render path: use UpdateLayeredWindow for per-pixel alpha transparency.
    /// Softbuffer's BitBlt doesn't support per-pixel alpha on layered windows.
    #[cfg(target_os = "windows")]
    pub fn render(
        &mut self,
        _surface: &mut Surface<Arc<Window>, Arc<Window>>,
        _width: u32,
        height: u32,
        units: &UnitManager,
        dock_height: u32,
    ) {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use windows::Win32::Graphics::Gdi::*;
        use windows::Win32::UI::WindowsAndMessaging::*;
        use windows::Win32::Foundation::{HWND, POINT, SIZE};

        // Clear canvas to fully transparent.
        self.canvas.data_mut().fill(0);

        // Draw all units.
        units.draw_all(&mut self.canvas, height, self.pixel_scale, dock_height);

        let Some(window) = &self.window else { return };
        let Ok(handle) = window.window_handle() else { return };
        let RawWindowHandle::Win32(wh) = handle.as_raw() else { return };
        let hwnd = HWND(wh.hwnd.get() as *mut _);

        let w = self.canvas.width() as i32;
        let h = self.canvas.height() as i32;

        unsafe {
            let hdc_screen = GetDC(HWND::default());
            let hdc_mem = CreateCompatibleDC(hdc_screen);

            // Create a 32-bit ARGB DIB section.
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h, // top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
            let hbmp = match CreateDIBSection(
                hdc_mem,
                &bmi,
                DIB_RGB_COLORS,
                &mut bits_ptr,
                None,
                0,
            ) {
                Ok(bmp) => bmp,
                Err(_) => {
                    let _ = DeleteDC(hdc_mem);
                    ReleaseDC(HWND::default(), hdc_screen);
                    return;
                }
            };
            let old_bmp = SelectObject(hdc_mem, HGDIOBJ(hbmp.0));

            // Copy canvas pixels to the DIB as premultiplied BGRA.
            let dst_slice = std::slice::from_raw_parts_mut(bits_ptr as *mut u32, (w * h) as usize);
            for (dst, src) in dst_slice.iter_mut().zip(self.canvas.pixels()) {
                // tiny_skia is premultiplied RGBA; Windows wants premultiplied BGRA.
                *dst = (src.alpha() as u32) << 24
                     | (src.red() as u32) << 16
                     | (src.green() as u32) << 8
                     | src.blue() as u32;
            }

            let size = SIZE { cx: w, cy: h };
            let pt_src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: 0, // AC_SRC_OVER
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: 1, // AC_SRC_ALPHA
            };
            let _ = UpdateLayeredWindow(
                hwnd,
                hdc_screen,
                None,
                Some(&size),
                hdc_mem,
                Some(&pt_src),
                windows::Win32::Foundation::COLORREF(0),
                Some(&blend),
                UPDATE_LAYERED_WINDOW_FLAGS(2), // ULW_ALPHA = 2
            );

            SelectObject(hdc_mem, old_bmp);
            let _ = DeleteObject(HGDIOBJ(hbmp.0));
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(HWND::default(), hdc_screen);
        }
    }

    /// Non-macOS/non-Windows render path: blit via softbuffer.
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    pub fn render(
        &mut self,
        surface: &mut Surface<Arc<Window>, Arc<Window>>,
        _width: u32,
        height: u32,
        units: &UnitManager,
        dock_height: u32,
    ) {
        self.canvas.data_mut().fill(0);
        units.draw_all(&mut self.canvas, height, self.pixel_scale, dock_height);
        let mut buffer = surface.buffer_mut().expect("Failed to get surface buffer");
        for (dst, src) in buffer.iter_mut().zip(self.canvas.pixels()) {
            let a = src.alpha();
            if a == 0 {
                *dst = 0u32;
            } else {
                let r = (src.red() as u32 * 255 / a as u32).min(255);
                let g = (src.green() as u32 * 255 / a as u32).min(255);
                let b = (src.blue() as u32 * 255 / a as u32).min(255);
                *dst = (r << 16) | (g << 8) | b;
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
        let pixel_count = self.canvas.pixels().len();
        debug_assert_eq!(pixel_count, w * h, "Canvas pixel count mismatch");
        let buf_len = self.pixel_buf.len();
        for (i, px) in self.canvas.pixels().iter().enumerate() {
            let base = i * 4;
            if base + 3 >= buf_len {
                break;
            }
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
            let copy_len = (w * h * 4).min(self.pixel_buf.len());
            unsafe {
                std::ptr::copy_nonoverlapping(
                    self.pixel_buf.as_ptr(),
                    ctx_data_ptr as *mut u8,
                    copy_len,
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
        self.window = Some(window.clone());
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

    /// Non-macOS: store window reference for UpdateLayeredWindow.
    #[cfg(not(target_os = "macos"))]
    pub fn set_window(&mut self, window: &Arc<Window>) {
        self.window = Some(window.clone());
    }
}
