use bitflags::bitflags;
use common::types::{GForce, InertialFrame};
use crate::define_peripheral;

define_peripheral! {
    Lsm6dsl:
        reg WhoAmI, addrs=0x0F, type=fixed, val=0x6A;
        reg CtrlReg1XL, addrs=0x10, type=config
            field OutputDataRateXL, default=PowerDown, shift=4
                flag PowerDown, 0b0000
                flag _1_6, 0b1011
                flag _12_5, 0b0001
            field ScaleXL, default=_2G, shift=2
            // ???
            field LPF1_BwSelectLPF1, default=, shift=1
            field BwXL, default=, shift=0
        reg CtrlReg2G, addrs=0x11, type=config
            field OutputDataRateG, default=PowerDown, shift=4
            field ScaleG, default=_250DPS, shift=2
            field Scale_125G, default=Disable, shift=1
        reg CtrlReg3C, addrs=0x12, type=config
            field Reboot, default=NormalMode, shift=7
                flag Active, 0b1
                flag NormalMode, 0b0
            field BlockDataUpdate, default=Disable, shift=6
                flag Enable, 0b1
                flag Disable, 0b0
            field SpiMode, default=_4Wire, shift=3
                flag _4Wire, 0b0
                flag _3Wire, 0b1
            field SpiMode, default=_4Wire, shift=3
                flag _4Wire, 0b0
                flag _3Wire, 0b1
            field AutoInc, default=Enable, shift=2
                flag Enable, 0b1
                flag Disable, 0b0
            field Endianness, default=Big, shift=1
                flag Big, 0b0
                flag Little, 0b1
            field Reset, default=NormalMode, shift=0
                flag Active, 0b1
                flag NormalMode, 0b0
        reg CtrlReg4C, addrs=0x13, type=config
            field DenXL, default=Disable, shift=7
                flag Enable, 0b1
                flag Disable, 0b0
            field SleepG, default=Disable, shift=6
                flag Enable, 0b1
                flag Disable, 0b0
            // ???
            field DrdyMask, default=Disable, shift=3
                flag Enable, 0b1
                flag Disable, 0b0
            field I2C, default=Enable, shift=2
                flag Enable, 0b0
                flag Disable, 0b1
            field LPF1_SelectG, default=Enable, shift=1
                flag Enable, 0b1
                flag Disable, 0b0
        reg CtrlReg5C, addrs=0x14, type=config
            flag Rounding, default=None, shift=5
            // ??
            flag Den_LH, default=ActiveLow, shift=4
            flag SelfTestG, default=NormalMode, shift=2
            flag SelfTestXL, default=NormalMode, shift=0
        reg CtrlReg6C, addrs=0x15, type=config
            // ??
        reg CtrlReg7G, addrs=0x16, type=config
        reg CtrlReg8XL, addrs=0x17, type=config
        reg CtrlReg9XL, addrs=0x18, type=config
        reg CtrlReg10C, addrs=0x19, type=config
            field WristTilt, default=Disable, shift=7
                flag Enable, 0b1
                flag Disable, 0b0
            field Timer, default=Disable, shift=5
                flag Enable, 0b1
                flag Disable, 0b0
            field Pedometer, default=Disable, shift=4
                flag Enable, 0b1
                flag Disable, 0b0
            field Tilt, default=Disable, shift=3
                flag Enable, 0b1
                flag Disable, 0b0
            field EmbeddedFunc, default=Disable, shift=2
                flag Enable, 0b1
                flag Disable, 0b0
            field PedometerReset, default=Disable, shift=1
                flag Enable, 0b1
                flag Disable, 0b0
            field SignificantMotion, default=Disable, shift=0
                flag Enable, 0b1
                flag Disable, 0b0
        reg StatusReg, addrs=0x1E, type=val, data=StatusFlags;
        reg RawTempLReg, addrs=0x20, type=val, data=u8;
        reg RawTempHReg, addrs=0x21, type=val, data=u8;
        reg RawXL_GReg, addrs=0x22, type=val, data=u8;
        reg RawXH_GReg, addrs=0x23, type=val, data=u8;
        reg RawYL_GReg, addrs=0x24, type=val, data=u8;
        reg RawYH_GReg, addrs=0x25, type=val, data=u8;
        reg RawZL_GReg, addrs=0x26, type=val, data=u8;
        reg RawZH_GReg, addrs=0x27, type=val, data=u8;
        reg RawXL_XLReg, addrs=0x28, type=val, data=u8;
        reg RawXH_XLReg, addrs=0x29, type=val, data=u8;
        reg RawYL_XLReg, addrs=0x2A, type=val, data=u8;
        reg RawYH_XLReg, addrs=0x2B, type=val, data=u8;
        reg RawZL_XLReg, addrs=0x2C, type=val, data=u8;
        reg RawZH_XLReg, addrs=0x2D, type=val, data=u8;
        reg RawZH_XLReg, addrs=0x2D, type=val, data=u8;
        reg Frame, addrs=0x22, doc="Reads a InertialFrame", type=raw, flags=(Scale, Endianness);
}

impl<ScaleG_: lsm6dsl::ScaleG, ScaleXL_: lsm6dsl::ScaleXL, Scale_125G_: lis6dsl::Scale_125G, Endianness_: lsm6dsl::Endianness> ReadableRegister for lsm6dsl::mag_vec::MagVec<Scale_, Endianness_> {
    type Data = InertialFrame;

    fn read(dev: &mut impl Device) -> anyhow::Result<Self::Data> {
        let buffer = &mut [0; 6];
        dev.read(<Self as Register>::ADDRESS, buffer).context("Could not read magnetometer data")?;

        let (values, _) = buffer.as_chunks();
        let adapter = match Endianness_::ENDIANNESS {
            lsm6dsl::EndiannessFlags::Big => i16::from_be_bytes,
            lsm6dsl::EndiannessFlags::Little => i16::from_le_bytes,
        };

        todo!()

        Ok(MagFrame {
            mag_x: GForce((adapter)(values[0]) as f64 / Scale_::SENSITIVITY),
            mag_y: GForce((adapter)(values[1]) as f64 / Scale_::SENSITIVITY),
            mag_z: GForce((adapter)(values[2]) as f64 / Scale_::SENSITIVITY),
        })
    }
}

bitflags! {
    pub struct StatusFlags: u8 {
        const TDA = 0b0000_0100;
        const GDA = 0b0000_0010;
        const XLDA = 0b0000_0001;
    }
}