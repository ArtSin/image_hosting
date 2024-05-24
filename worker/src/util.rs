use image::{flat::SampleLayout, ImageBuffer, Pixel};
use ndarray::{Array3, ShapeBuilder};

pub trait ToNdarray3 {
    type Out;

    fn into_ndarray3(self) -> Self::Out;
}

impl<P> ToNdarray3 for ImageBuffer<P, Vec<P::Subpixel>>
where
    P: Pixel + 'static,
{
    type Out = Array3<P::Subpixel>;

    fn into_ndarray3(self) -> Self::Out {
        let SampleLayout {
            channels,
            channel_stride,
            height,
            height_stride,
            width,
            width_stride,
        } = self.sample_layout();
        let shape = (channels as usize, height as usize, width as usize);
        let strides = (channel_stride, height_stride, width_stride);
        Array3::from_shape_vec(shape.strides(strides), self.into_raw()).unwrap()
    }
}
