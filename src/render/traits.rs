use crate::AppError;

pub trait Render: Send {
    fn load_src_data(&mut self, path: &str) -> Result<(), AppError>;
    fn render_frame(&self) -> Result<(), AppError>;
    fn next_frame(&mut self) -> Result<(), AppError>;
}

impl Render for () {
    fn load_src_data(&mut self, path: &str) -> Result<(), AppError> {
        let _ = path;
        Ok(())
    }

    fn render_frame(&self) -> Result<(), AppError> {
        Ok(())
    }

    fn next_frame(&mut self) -> Result<(), AppError> {
        Ok(())
    }
}
