use std::{cell::RefCell, time::Duration};

use anyhow::{bail, Context};
use common::types::{Movement, Percent, PidConfig, PidController};
use egui::epaint::ahash::HashMap;
use opencv::{
    core::{self as cvcore, Scalar, VecN},
    core::{Point, Size, BORDER_DEFAULT},
    imgproc::{
        self, COLOR_BGR2Lab, CHAIN_APPROX_SIMPLE, COLOR_BGR2RGBA, COLOR_GRAY2RGBA, LINE_8,
        MARKER_CROSS, RETR_LIST,
    },
    prelude::*,
    types::VectorOfVectorOfPoint,
};

pub type Mats = HashMap<MatId, RefCell<Mat>>;
pub type SourceFn = Box<dyn FnMut(&mut Mats) -> anyhow::Result<bool>>;
pub type ProcessorFn = Box<dyn FnMut(&mut Mats) -> anyhow::Result<Movement>>;
pub type PipelineProto = Vec<PipelineStage>;

/// Repersents a image created in the image process pipeline
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum MatId {
    Raw,
    Blur,
    Lab,
    Mask,
    ButtonOverlay,
}

impl MatId {
    pub fn conversion_code(&self) -> i32 {
        match self {
            MatId::Raw => COLOR_BGR2RGBA,
            MatId::Blur => COLOR_BGR2RGBA,
            MatId::Lab => COLOR_BGR2RGBA,
            MatId::Mask => COLOR_GRAY2RGBA,
            MatId::ButtonOverlay => COLOR_BGR2RGBA,
        }
    }
}

/// Repersents a stage in the image processing pipeline
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum PipelineStage {
    PushButton,
}

impl PipelineStage {
    pub fn construct(&self) -> ProcessorFn {
        match self {
            PipelineStage::PushButton => {
                let period = Duration::from_secs_f64(1.0 / 30.0);

                let mut button_tracker = TrackButton {
                    pid_config: PidConfig {
                        kp: 0.003,
                        ki: 0.0,
                        kd: 0.0,
                        max_integral: 0.0,
                    },
                    pid_x: PidController::new(period),
                    pid_y: PidController::new(period),
                    target_x: 0.0,
                    target_y: 0.5,
                    color: (130, 180, 130),
                    color_varance: (90, 30, 40),
                };

                Box::new(move |mats| button_tracker.update(mats))
            }
        }
    }

    pub fn all() -> PipelineProto {
        vec![Self::PushButton]
    }
}

pub struct TrackButton {
    pid_config: PidConfig,
    pid_x: PidController,
    pid_y: PidController,

    target_x: f64,
    target_y: f64,

    color: (u8, u8, u8),
    color_varance: (u8, u8, u8),
}

impl TrackButton {
    pub fn update(&mut self, mats: &mut Mats) -> anyhow::Result<Movement> {
        mats.entry(MatId::Blur).or_default();
        mats.entry(MatId::Lab).or_default();
        mats.entry(MatId::Mask).or_default();
        mats.entry(MatId::ButtonOverlay).or_default();

        let image = mats.get(&MatId::Raw).context("No raw frame")?;

        let blur = mats.get(&MatId::Blur).unwrap();
        imgproc::gaussian_blur(
            &*image.borrow(),
            &mut *blur.borrow_mut(),
            Size::new(25, 25),
            0.0,
            0.0,
            BORDER_DEFAULT,
        )
        .context("blur")?;

        let lab = mats.get(&MatId::Lab).unwrap();
        imgproc::cvt_color(&*blur.borrow(), &mut *lab.borrow_mut(), COLOR_BGR2Lab, 0)
            .context("Convert colors to lab")?;

        let lower_bound: VecN<_, 3> = [
            self.color.0 - self.color_varance.0,
            self.color.1 - self.color_varance.1,
            self.color.2 - self.color_varance.2,
        ]
        .into();
        let upper_bound: VecN<_, 3> = [
            self.color.0 + self.color_varance.0,
            self.color.1 + self.color_varance.1,
            self.color.2 + self.color_varance.2,
        ]
        .into();
        let mask = mats.get(&MatId::Mask).unwrap();
        cvcore::in_range(
            &*lab.borrow(),
            &lower_bound,
            &upper_bound,
            &mut *mask.borrow_mut(),
        )
        .context("In range")?;

        let mut contours = VectorOfVectorOfPoint::new();
        imgproc::find_contours(
            &*mask.borrow(),
            &mut contours,
            RETR_LIST,
            CHAIN_APPROX_SIMPLE,
            Point::default(),
        )
        .context("Find contours")?;

        let overlay = mats.get(&MatId::ButtonOverlay).unwrap();
        image
            .borrow()
            .copy_to(&mut *overlay.borrow_mut())
            .context("Copy image to overlay")?;
        imgproc::draw_contours(
            &mut *overlay.borrow_mut(),
            &contours,
            -1,
            Scalar::new(0.0, 255.0, 0.0, 0.0),
            2,
            LINE_8,
            &cvcore::no_array(),
            i32::MAX,
            Point::default(),
        )
        .context("draw contours")?;

        let contour = contours
            .iter()
            .map(|it| (imgproc::contour_area(&it, false).unwrap_or_default(), it))
            .max_by(|a, b| f64::total_cmp(&a.0, &b.0))
            .context("No contour found")?;

        let moments = imgproc::moments(&contour.1, false).context("Get moments")?;

        if moments.m00 == 0.0 {
            bail!("Selected bad contour");
        }

        let center_x = moments.m10 / moments.m00;
        let center_y = moments.m01 / moments.m00;

        imgproc::draw_marker(
            &mut *overlay.borrow_mut(),
            Point::new(center_x as i32, center_y as i32),
            Scalar::new(255.0, 0.0, 0.0, 0.0),
            MARKER_CROSS,
            20,
            2,
            LINE_8,
        )
        .context("Draw marker")?;

        let image_size = image.borrow().size().context("Get mat size")?;
        let target_x = image_size.width as f64 / 2.0 * (self.target_x + 1.0);
        let target_y = image_size.height as f64 / 2.0 * (self.target_y + 1.0);

        imgproc::draw_marker(
            &mut *overlay.borrow_mut(),
            Point::new(target_x as i32, target_y as i32),
            Scalar::new(255.0, 255.0, 0.0, 0.0),
            MARKER_CROSS,
            20,
            2,
            LINE_8,
        )
        .context("Draw marker")?;

        let delta_x = target_x - center_x;
        let delta_y = target_y - center_y;

        let pid_result_x = self.pid_x.update(delta_x, self.pid_config);
        let pid_result_y = self.pid_y.update(delta_y, self.pid_config);

        let max_correction = 0.30;
        let correction_x = pid_result_x
            .correction()
            .clamp(-max_correction, max_correction);
        let correction_y = pid_result_y
            .correction()
            .clamp(-max_correction, max_correction);

        Ok(Movement {
            x: Percent::new(-correction_x),
            y: Percent::new(0.2),
            z: Percent::new(correction_y),
            ..Default::default()
        })
    }
}
