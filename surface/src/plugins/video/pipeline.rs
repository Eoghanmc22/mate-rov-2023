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
    RotateIntermediate,
    Camera,
    Blur,
    Lab,
    Mask,
    ButtonOverlay,
}

impl MatId {
    pub fn conversion_code(&self) -> i32 {
        match self {
            MatId::RotateIntermediate => COLOR_BGR2RGBA,
            MatId::Camera => COLOR_BGR2RGBA,
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
    FlyTransect,
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
                    target_y: 0.4,
                    color: (100, 160, 140),
                    color_varance: (20, 20, 20),
                };

                Box::new(move |mats| button_tracker.update(mats))
            }
            PipelineStage::FlyTransect => {
                let period = Duration::from_secs_f64(1.0 / 30.0);

                let mut fly_transect = FlyTransect {
                    lateral_pid_config: PidConfig {
                        kp: 0.003,
                        ki: 0.0,
                        kd: 0.0,
                        max_integral: 0.0,
                    },
                    rot_pid_config: PidConfig {
                        kp: 0.03,
                        ki: 0.0,
                        kd: 0.0,
                        max_integral: 0.0,
                    },
                    pid_x: PidController::new(period),
                    pid_z: PidController::new(period),
                    pid_z_rot: PidController::new(period),
                    target_x: 0.0,
                    target_dist: 50.0,
                    target_theta: 0.0,
                    color: (190, 109, 99),
                    color_varance: (18, 6, 14),
                };

                Box::new(move |mats| fly_transect.update(mats))
            }
        }
    }

    pub fn all() -> PipelineProto {
        vec![Self::PushButton, Self::FlyTransect]
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

        let image = mats.get(&MatId::Camera).context("No raw frame")?;

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

pub struct FlyTransect {
    lateral_pid_config: PidConfig,
    rot_pid_config: PidConfig,

    pid_x: PidController,
    pid_z: PidController,
    pid_z_rot: PidController,

    target_x: f64,
    target_dist: f64,
    target_theta: f64,

    color: (u8, u8, u8),
    color_varance: (u8, u8, u8),
}

impl FlyTransect {
    pub fn update(&mut self, mats: &mut Mats) -> anyhow::Result<Movement> {
        mats.entry(MatId::Blur).or_default();
        mats.entry(MatId::Lab).or_default();
        mats.entry(MatId::Mask).or_default();
        mats.entry(MatId::ButtonOverlay).or_default();

        let image = mats.get(&MatId::Camera).context("No raw frame")?;

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

        let mut contour_a = (None, 0.0);
        let mut contour_b = (None, 0.0);

        for contour in contours {
            if let Ok(area) = imgproc::contour_area(&contour, false) {
                if contour_a.1 < area {
                    contour_b = contour_a;
                    contour_a = (Some(contour), area);
                } else if contour_b.1 < area {
                    contour_b = (Some(contour), area);
                }
            }
        }

        let moments_a = imgproc::moments(&contour_a.1, false).context("Get moments")?;
        let moments_b = imgproc::moments(&contour_b.1, false).context("Get moments")?;

        if moments_a.m00 == 0.0 {
            bail!("Selected bad contour");
        }

        if moments_b.m00 == 0.0 {
            bail!("Selected bad contour");
        }

        let a_x = moments_a.m10 / moments_a.m00;
        let a_y = moments_a.m01 / moments_a.m00;
        let b_x = moments_a.m10 / moments_a.m00;
        let b_y = moments_a.m01 / moments_a.m00;

        let center_x = (a_x + a_y) / 2.0;
        let center_y = (b_x + b_y) / 2.0;

        let d_x = b_x - a_x;
        let d_y = b_y - a_y;
        let theta = d_y.atan2(d_x);

        imgproc::line(
            &mut *overlay.borrow_mut(),
            Point::new(a_x as i32, a_y as i32),
            Point::new(b_x as i32, b_y as i32),
            Scalar::new(255.0, 0.0, 0.0, 0.0),
            20,
            LINE_8,
            0,
        )
        .context("Draw line")?;
        imgproc::draw_marker(
            &mut *overlay.borrow_mut(),
            Point::new(center_x as i32, center_y as i32),
            Scalar::new(0.0, 255.0, 0.0, 0.0),
            MARKER_CROSS,
            20,
            2,
            LINE_8,
        )
        .context("Draw marker")?;

        let image_size = image.borrow().size().context("Get mat size")?;
        let target_x = image_size.width as f64 / 2.0 * (self.target_x + 1.0);
        let target_y = image_size.height as f64 / 2.0;

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
        let delta_z = self.target_dist * self.target_dist - (d_x * d_x + d_y * d_y);
        let delta_z_rot = self.target_theta - theta;

        let pid_result_x = self.pid_x.update(delta_x, self.lateral_pid_config);
        let pid_result_z = self.pid_z.update(delta_z, self.lateral_pid_config);
        let pid_result_z_rot = self.pid_z_rot.update(delta_z_rot, self.rot_pid_config);

        let max_correction = 0.30;
        let correction_x = pid_result_x
            .correction()
            .clamp(-max_correction, max_correction);
        let correction_y = (max_correction - theta.abs() * 0.1).max(0.0);
        let correction_z = pid_result_z
            .correction()
            .clamp(-max_correction, max_correction);
        let correction_z_rot = pid_result_z_rot
            .correction()
            .clamp(-max_correction, max_correction);

        Ok(Movement {
            x: Percent::new(-correction_x),
            y: Percent::new(correction_y),
            z: Percent::new(correction_z),
            z_rot: Percent::new(correction_z_rot),
            ..Default::default()
        })
    }
}
