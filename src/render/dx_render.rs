use super::Render;
use crate::{AppError, window::WindowHandle};
use log::debug;
use windows::{
    Win32::{
        Foundation::{GENERIC_READ, RECT},
        Graphics::{
            Direct2D::{Common::*, *},
            Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
            Imaging::*,
        },
        System::Com::{CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize},
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::GetClientRect},
    },
    core::HSTRING,
};

struct GifFrame {
    bitmap: ID2D1Bitmap,
}

pub struct DxRender {
    frames: Vec<GifFrame>,
    pub current_frame: usize,
    render_target: ID2D1HwndRenderTarget,
}

unsafe impl Send for DxRender {}

impl DxRender {
    pub fn new(hwnd: WindowHandle) -> Result<Self, AppError> {
        let render_target = get_render_target(hwnd)?;
        Ok(DxRender {
            frames: Vec::new(),
            current_frame: 0,
            render_target,
        })
    }
    fn create_d2d_bitmap_from_frame(&self, wic_factory: &IWICImagingFactory, decoder: &IWICBitmapDecoder, index: u32) -> Result<ID2D1Bitmap, AppError> {
        let wic_frame = unsafe { decoder.GetFrame(index)? };
        let converter = unsafe { wic_factory.CreateFormatConverter()? };

        unsafe { converter.Initialize(&wic_frame, &GUID_WICPixelFormat32bppPBGRA, WICBitmapDitherTypeNone, None, 0.0, WICBitmapPaletteTypeCustom)? };

        match unsafe { self.render_target.CreateBitmapFromWicBitmap(&converter, None) } {
            Ok(bitmap) => Ok(bitmap),
            _ => Err(AppError("Get bitmap error".into())),
        }
    }
}

impl Render for DxRender {
    fn load_src_data(&mut self, path: &str) -> Result<(), crate::AppError> {
        let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        let wic_factory: IWICImagingFactory = unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)? };
        let decoder = unsafe { wic_factory.CreateDecoderFromFilename(&HSTRING::from(path), None, GENERIC_READ, WICDecodeMetadataCacheOnLoad)? };
        let frame_count = unsafe { decoder.GetFrameCount()? };
        debug!("Get frame count: {:?}", frame_count);
        for i in 0..frame_count {
            let frame_bitmap = self.create_d2d_bitmap_from_frame(&wic_factory, &decoder, i)?;
            let gif_frame = GifFrame { bitmap: frame_bitmap };
            self.frames.push(gif_frame);
        }
        let _ = unsafe { CoUninitialize() };
        Ok(())
    }

    fn render_frame(&self) -> Result<(), AppError> {
        unsafe {
            self.render_target.BeginDraw();
            self.render_target.Clear(None);

            if let Some(frame) = self.frames.get(self.current_frame) {
                let mut rc = RECT::default();
                GetClientRect(self.render_target.GetHwnd(), &mut rc)?;
                let hwnd = self.render_target.GetHwnd();
                let dpi = GetDpiForWindow(hwnd) as f32;
                let scale = dpi / 96.0;

                let win_width = (rc.right - rc.left) as f32 / scale;
                let win_height = (rc.bottom - rc.top) as f32 / scale;

                let bmp = &frame.bitmap;
                let bmp_size = bmp.GetSize();
                let bmp_width = bmp_size.width;
                let bmp_height = bmp_size.height;

                let scale = (win_width / bmp_width).min(win_height / bmp_height);
                let draw_width = bmp_width * scale;
                let draw_height = bmp_height * scale;
                let left = (win_width - draw_width) / 2.0;
                let top = (win_height - draw_height) / 2.0;
                let dest_rect = D2D_RECT_F {
                    left,
                    top,
                    right: left + draw_width,
                    bottom: top + draw_height,
                };

                self.render_target.DrawBitmap(bmp, Some(&dest_rect), 1.0, D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, None);
            }
            let (mut t1, mut t2) = (0u64, 0u64);
            self.render_target
                .EndDraw(Some(&mut t1), Some(&mut t2))
                .map_err(|e| AppError(format!("EndDraw failed: {}, find err1 {}, err2 {}", e, t1, t2)))
        }
    }

    fn next_frame(&mut self) -> Result<(), AppError> {
        self.current_frame = (self.current_frame + 1) % self.frames.len();
        Ok(())
    }
}

pub fn get_render_target(hwnd: WindowHandle) -> Result<ID2D1HwndRenderTarget, AppError> {
    let hwnd = hwnd.0;
    let d2d_factory: ID2D1Factory = unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)? };

    let mut rc = RECT::default();
    unsafe { GetClientRect(hwnd, &mut rc)? };

    let render_target = unsafe {
        d2d_factory.CreateHwndRenderTarget(
            &D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_HARDWARE,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                ..Default::default()
            },
            &D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd,
                pixelSize: D2D_SIZE_U {
                    width: (rc.right - rc.left) as u32,
                    height: (rc.bottom - rc.top) as u32,
                },
                ..Default::default()
            },
        )?
    };

    Ok(render_target)
}
