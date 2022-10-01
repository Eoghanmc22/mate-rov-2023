use crate::peripheral::spi::Device;

// TODO WRITE TESTS
// - Data sheet has a list of default values for each reg
// TODO Write Comments

pub const LIS3MDL_ADDRESS: u8     = 0x1C;

pub const LIS3MDL_WHO_AM_I: u8    = 0x0F;

pub const LIS3MDL_CTRL_REG1: u8   = 0x20;
pub const LIS3MDL_CTRL_REG2: u8   = 0x21;
pub const LIS3MDL_CTRL_REG3: u8   = 0x22;
pub const LIS3MDL_CTRL_REG4: u8   = 0x23;
pub const LIS3MDL_CTRL_REG5: u8   = 0x24;

pub const LIS3MDL_STATUS_REG: u8  = 0x27;

pub const LIS3MDL_OUT_BLOCK: u8   = 0x28;
pub const LIS3MDL_OUT_X_L: u8     = 0x28;
pub const LIS3MDL_OUT_X_H: u8     = 0x29;
pub const LIS3MDL_OUT_Y_L: u8     = 0x2A;
pub const LIS3MDL_OUT_Y_H: u8     = 0x2B;
pub const LIS3MDL_OUT_Z_L: u8     = 0x2C;
pub const LIS3MDL_OUT_Z_H: u8     = 0x2D;

pub const LIS3MDL_TEMP_OUT_L: u8  = 0x2E;
pub const LIS3MDL_TEMP_OUT_H: u8  = 0x2F;

pub const LIS3MDL_INT_CFG: u8     = 0x30;
pub const LIS3MDL_INT_SRC: u8     = 0x31;
pub const LIS3MDL_INT_THS_L: u8   = 0x32;
pub const LIS3MDL_INT_THS_H: u8   = 0x33;


pub trait WriteableRegister {
    fn write(&self, dest: &mut impl Device);
}

trait Field<T> {
    const FIELD: T;
    const SHIFT: usize;
}

macro_rules! write_fields {
    ($addrs:expr, $device:ident: $( $field:ty ),+) => {
        let reg: u8 = $( (<$field as Field<_>>::FIELD).bits() << <$field as Field<_>>::SHIFT | )* 0;
        Device::write_byte($device, $addrs, reg);
    };
}

//TODO implement useful stuff lol
pub struct Lis3mdl {
    // TODO who_am_i
    crtl_reg_1: crtl_reg_1::CrtlReg1,
    crtl_reg_2: crtl_reg_2::CrtlReg2,
    crtl_reg_3: crtl_reg_3::CrtlReg3,
    // TODO crtl_reg_4
    // TODO crtl_reg_5
    // TODO status_reg

    // TODO out block
    // TODO interrupts
}

pub mod crtl_reg_1 {
    use std::marker::PhantomData;
    use bitflags::bitflags;
    use crate::peripheral::lis3mdl::{Field, WriteableRegister};
    use crate::peripheral::spi::Device;

    #[derive(Copy, Clone)]
    pub struct CrtlReg1<Temperature = TemperatureDisable, Performance = LowPerformance, OutputDataRate = OutputDataRate10_0, FastOdr = FastOdrDisable, SelfTest = SelfTestDisable>(PhantomData<Temperature>, PhantomData<Performance>, PhantomData<OutputDataRate>, PhantomData<FastOdr>, PhantomData<SelfTest>);

    impl<Temperature_: Temperature, Performance_: Performance, OutputDataRate_: OutputDataRate, FastOdr_: FastOdr, SelfTest_: SelfTest> CrtlReg1<Temperature_, Performance_, OutputDataRate_, FastOdr_, SelfTest_>  {
        pub const ADDRESS: u8 = 0x20;

        pub fn new() -> Self {
            Self(Default::default(), Default::default(), Default::default(), Default::default(), Default::default())
        }
    }

    impl<Temperature_: Temperature, Performance_: Performance, OutputDataRate_: OutputDataRate, FastOdr_: FastOdr, SelfTest_: SelfTest> WriteableRegister for CrtlReg1<Temperature_, Performance_, OutputDataRate_, FastOdr_, SelfTest_>  {
        fn write(&self, dest: &mut impl Device) {
            write_fields!(Self::ADDRESS, dest: Temperature_, Performance_, OutputDataRate_, FastOdr_, SelfTest_);
        }
    }


    bitflags! {
        pub struct TempatureFlags: u8 {
            const ENABLE  = 0b1;
            const DISABLE = 0b0;
        }
    }

    pub trait Temperature {
        const TEMPERATURE: TempatureFlags;
        const SHIFT: usize = 7;
    }

    impl<T> Field<TempatureFlags> for T where T: Temperature {
        const FIELD: TempatureFlags = <T as Temperature>::TEMPERATURE;
        const SHIFT: usize = <T as Temperature>::SHIFT;
    }

    pub struct TemperatureEnable;
    pub struct TemperatureDisable;
    impl Temperature for TemperatureEnable { const TEMPERATURE: TempatureFlags = TempatureFlags::ENABLE; }
    impl Temperature for TemperatureDisable { const TEMPERATURE: TempatureFlags = TempatureFlags::DISABLE; }


    bitflags! {
        pub struct PerformanceFlags: u8 {
            const LOW_PERFORMANCE        = 0b00;
            const MEDIUM_PERFORMANCE     = 0b01;
            const HIGH_PERFORMANCE       = 0b10;
            const ULTRA_HIGH_PERFORMANCE = 0b11;
        }
    }

    pub trait Performance {
        const PERFORMANCE: PerformanceFlags;
        const SHIFT: usize = 5;
    }

    impl<T> Field<PerformanceFlags> for T where T: Performance {
        const FIELD: PerformanceFlags = <T as Performance>::PERFORMANCE;
        const SHIFT: usize = <T as Performance>::SHIFT;
    }

    pub struct LowPerformance;
    pub struct MediumPerformance;
    pub struct HighPerformance;
    pub struct UltraHighPerformance;
    impl Performance for LowPerformance { const PERFORMANCE: PerformanceFlags = PerformanceFlags::LOW_PERFORMANCE; }
    impl Performance for MediumPerformance { const PERFORMANCE: PerformanceFlags = PerformanceFlags::MEDIUM_PERFORMANCE; }
    impl Performance for HighPerformance { const PERFORMANCE: PerformanceFlags = PerformanceFlags::HIGH_PERFORMANCE; }
    impl Performance for UltraHighPerformance { const PERFORMANCE: PerformanceFlags = PerformanceFlags::ULTRA_HIGH_PERFORMANCE; }


    bitflags! {
        pub struct OutputDataRateFlags: u8 {
            const RATE_0_625 = 0b000;
            const RATE_1_25  = 0b001;
            const RATE_2_5   = 0b010;
            const RATE_5_0   = 0b011;
            const RATE_10_0  = 0b100;
            const RATE_20_0  = 0b101;
            const RATE_40_0  = 0b110;
            const RATE_80_0  = 0b111;
        }
    }

    pub trait OutputDataRate {
        const ODR: OutputDataRateFlags;
        const SHIFT: usize = 2;
    }

    impl<T> Field<OutputDataRateFlags> for T where T: OutputDataRate {
        const FIELD: OutputDataRateFlags = <T as OutputDataRate>::ODR;
        const SHIFT: usize = <T as OutputDataRate>::SHIFT;
    }

    pub struct OutputDataRate0_625;
    pub struct OutputDataRate1_25;
    pub struct OutputDataRate2_5;
    pub struct OutputDataRate5_0;
    pub struct OutputDataRate10_0;
    pub struct OutputDataRate20_0;
    pub struct OutputDataRate40_0;
    pub struct OutputDataRate80_0;
    impl OutputDataRate for OutputDataRate0_625 { const ODR: OutputDataRateFlags = OutputDataRateFlags::RATE_0_625; }
    impl OutputDataRate for OutputDataRate1_25 { const ODR: OutputDataRateFlags = OutputDataRateFlags::RATE_0_625; }
    impl OutputDataRate for OutputDataRate2_5 { const ODR: OutputDataRateFlags = OutputDataRateFlags::RATE_2_5; }
    impl OutputDataRate for OutputDataRate5_0 { const ODR: OutputDataRateFlags = OutputDataRateFlags::RATE_5_0; }
    impl OutputDataRate for OutputDataRate10_0 { const ODR: OutputDataRateFlags = OutputDataRateFlags::RATE_10_0; }
    impl OutputDataRate for OutputDataRate20_0 { const ODR: OutputDataRateFlags = OutputDataRateFlags::RATE_20_0; }
    impl OutputDataRate for OutputDataRate40_0 { const ODR: OutputDataRateFlags = OutputDataRateFlags::RATE_40_0; }
    impl OutputDataRate for OutputDataRate80_0 { const ODR: OutputDataRateFlags = OutputDataRateFlags::RATE_80_0; }


    bitflags! {
        pub struct FastOdrFlags: u8 {
            const ENABLE  = 0b1;
            const DISABLE = 0b0;
        }
    }

    pub trait FastOdr {
        const FAST_ODR: FastOdrFlags;
        const SHIFT: usize = 1;
    }

    impl<T> Field<FastOdrFlags> for T where T: FastOdr {
        const FIELD: FastOdrFlags = <T as FastOdr>::FAST_ODR;
        const SHIFT: usize = <T as FastOdr>::SHIFT;
    }

    pub struct FastOdrEnable;
    pub struct FastOdrDisable;
    impl FastOdr for FastOdrEnable { const FAST_ODR: FastOdrFlags = FastOdrFlags::ENABLE; }
    impl FastOdr for FastOdrDisable { const FAST_ODR: FastOdrFlags = FastOdrFlags::DISABLE; }


    bitflags! {
        pub struct SelfTestFlags: u8 {
            const ENABLE  = 0b1;
            const DISABLE = 0b0;
        }
    }

    pub trait SelfTest {
        const SELF_TEST: SelfTestFlags;
        const SHIFT: usize = 0;
    }

    impl<T> Field<SelfTestFlags> for T where T: SelfTest {
        const FIELD: SelfTestFlags = <T as SelfTest>::SELF_TEST;
        const SHIFT: usize = <T as SelfTest>::SHIFT;
    }

    pub struct SelfTestEnable;
    pub struct SelfTestDisable;
    impl SelfTest for SelfTestEnable { const SELF_TEST: SelfTestFlags = SelfTestFlags::ENABLE; }
    impl SelfTest for SelfTestDisable { const SELF_TEST: SelfTestFlags = SelfTestFlags::DISABLE; }
}

pub mod ctrl_reg_2 {
    use std::marker::PhantomData;
    use bitflags::bitflags;
    use crate::peripheral::lis3mdl::{Field, WriteableRegister};
    use crate::peripheral::spi::Device;

    #[derive(Copy, Clone)]
    pub struct CrtlReg2<Scale = Scale4Gauss, Reboot = RebootNormalMode, SoftReset = SoftResetNormalMode>(PhantomData<Scale>, PhantomData<Reboot>, PhantomData<SoftReset>);

    impl<Scale_: Scale, Reboot_: Reboot, SoftReset_: SoftReset> CrtlReg2<Scale_, Reboot_, SoftReset_>  {
        pub const ADDRESS: u8 = 0x21;

        pub fn new() -> Self {
            Self(Default::default(), Default::default(), Default::default())
        }
    }

    impl<Scale_: Scale, Reboot_: Reboot, SoftReset_: SoftReset> WriteableRegister for CrtlReg2<Scale_, Reboot_, SoftReset_>  {
        fn write(&self, dest: &mut impl Device) {
            write_fields!(Self::ADDRESS, dest: Scale_, Reboot_, SoftReset_);
        }
    }


    bitflags! {
        pub struct ScaleFlags: u8 {
            const SCALE_4_GAUSS   = 0b00;
            const SCALE_8_GAUSS   = 0b01;
            const SCALE_12_GAUSS  = 0b10;
            const SCALE_16_GAUSS  = 0b11;
        }
    }

    pub trait Scale {
        const SCALE: ScaleFlags;
        const SHIFT: usize = 5;
    }

    impl<T> Field<ScaleFlags> for T where T: Scale {
        const FIELD: ScaleFlags = <T as Scale>::SCALE;
        const SHIFT: usize = <T as Scale>::SHIFT;
    }

    pub struct Scale4Gauss;
    pub struct Scale8Gauss;
    pub struct Scale12Gauss;
    pub struct Scale16Gauss;
    impl Scale for Scale4Gauss { const SCALE: ScaleFlags = ScaleFlags::SCALE_4_GAUSS; }
    impl Scale for Scale8Gauss { const SCALE: ScaleFlags = ScaleFlags::SCALE_8_GAUSS; }
    impl Scale for Scale12Gauss { const SCALE: ScaleFlags = ScaleFlags::SCALE_12_GAUSS; }
    impl Scale for Scale16Gauss { const SCALE: ScaleFlags = ScaleFlags::SCALE_16_GAUSS; }


    bitflags! {
        pub struct RebootFlags: u8 {
            const NORMAL_MODE = 0b0;
            const REBOOT      = 0b1;
        }
    }

    pub trait Reboot {
        const REBOOT: RebootFlags;
        const SHIFT: usize = 3;
    }

    impl<T> Field<RebootFlags> for T where T: Reboot {
        const FIELD: RebootFlags = <T as Reboot>::REBOOT;
        const SHIFT: usize = <T as Reboot>::SHIFT;
    }

    pub struct RebootNormalMode;
    pub struct RebootMemory;
    impl Reboot for RebootNormalMode { const REBOOT: RebootFlags = RebootFlags::NORMAL_MODE; }
    impl Reboot for RebootMemory { const REBOOT: RebootFlags = RebootFlags::REBOOT; }


    bitflags! {
        pub struct SoftResetFlags: u8 {
            const NORMAL_MODE = 0b0;
            const RESET       = 0b1;
        }
    }

    pub trait SoftReset {
        const SOFT_RESET: SoftResetFlags;
        const SHIFT: usize = 2;
    }

    impl<T> Field<SoftResetFlags> for T where T: SoftReset {
        const FIELD: SoftResetFlags = <T as SoftReset>::SOFT_RESET;
        const SHIFT: usize = <T as SoftReset>::SHIFT;
    }

    pub struct SoftResetNormalMode;
    pub struct SoftResetRegisters;
    impl SoftReset for SoftResetNormalMode { const SOFT_RESET: SoftResetFlags = SoftResetFlags::RESET; }
    impl SoftReset for SoftResetRegisters { const SOFT_RESET: SoftResetFlags = SoftResetFlags::NORMAL_MODE; }
}

pub mod ctrl_reg_3 {
    use std::marker::PhantomData;
    use bitflags::bitflags;
    use crate::peripheral::lis3mdl::{Field, WriteableRegister};
    use crate::peripheral::spi::Device;

    #[derive(Copy, Clone)]
    pub struct CrtlReg3<LowPower = LowPowerDisable, SpiMode = SpiMode4Wire, OperatingMode = OperatingModePowerDown>(PhantomData<LowPower>, PhantomData<SpiMode>, PhantomData<OperatingMode>);

    impl<LowPower_: LowPower, SpiMode_: SpiMode, OperatingMode_: OperatingMode> CrtlReg3<LowPower_, SpiMode_, OperatingMode_>  {
        pub const ADDRESS: u8 = 0x22;

        pub fn new() -> Self {
            Self(Default::default(), Default::default(), Default::default())
        }
    }

    impl<LowPower_: LowPower, SpiMode_: SpiMode, OperatingMode_: OperatingMode> WriteableRegister for CrtlReg3<LowPower_, SpiMode_, OperatingMode_>  {
        fn write(&self, dest: &mut impl Device) {
            write_fields!(Self::ADDRESS, dest: LowPower_, SpiMode_, OperatingMode_);
        }
    }


    bitflags! {
        pub struct LowPowerFlags: u8 {
            const ENABLE  = 0b1;
            const DISABLE = 0b0;
        }
    }

    pub trait LowPower {
        const LOW_POWER: LowPowerFlags;
        const SHIFT: usize = 5;
    }

    impl<T> Field<LowPowerFlags> for T where T: LowPower {
        const FIELD: LowPowerFlags = <T as LowPower>::LOW_POWER;
        const SHIFT: usize = <T as LowPower>::SHIFT;
    }

    pub struct LowPowerEnable;
    pub struct LowPowerDisable;
    impl LowPower for LowPowerEnable { const LOW_POWER: LowPowerFlags = LowPowerFlags::ENABLE; }
    impl LowPower for LowPowerDisable { const LOW_POWER: LowPowerFlags = LowPowerFlags::DISABLE; }


    bitflags! {
        pub struct SpiModeFlags: u8 {
            const SPI_4_WIRE = 0b0;
            const SPI_3_WIRE = 0b1;
        }
    }

    pub trait SpiMode {
        const SPI_MODE: SpiModeFlags;
        const SHIFT: usize = 2;
    }

    impl<T> Field<SpiModeFlags> for T where T: SpiMode {
        const FIELD: SpiModeFlags = <T as SpiMode>::SPI_MODE;
        const SHIFT: usize = <T as SpiMode>::SHIFT;
    }

    pub struct SpiMode4Wire;
    pub struct SpiMode3Wire;
    impl SpiMode for SpiMode4Wire { const SPI_MODE: SpiModeFlags = SpiModeFlags::SPI_4_WIRE; }
    impl SpiMode for SpiMode3Wire { const SPI_MODE: SpiModeFlags = SpiModeFlags::SPI_3_WIRE; }


    bitflags! {
        pub struct OperatingModeFlags: u8 {
            const CONTINUOUS_CONVERSION = 0b00;
            const SINGLE_CONVERSION     = 0b01;
            const POWER_DOWN            = 0b11;
        }
    }

    pub trait OperatingMode {
        const OPERATING_MODE: OperatingModeFlags;
        const SHIFT: usize = 0;
    }

    impl<T> Field<OperatingModeFlags> for T where T: OperatingMode {
        const FIELD: OperatingModeFlags = <T as OperatingMode>::OPERATING_MODE;
        const SHIFT: usize = <T as OperatingMode>::SHIFT;
    }

    pub struct OperatingModeContinuousConversion;
    pub struct OperatingModeSingleConversion;
    pub struct OperatingModePowerDown;
    impl OperatingMode for OperatingModeContinuousConversion { const OPERATING_MODE: OperatingModeFlags = OperatingModeFlags::CONTINUOUS_CONVERSION; }
    impl OperatingMode for OperatingModeSingleConversion { const OPERATING_MODE: OperatingModeFlags = OperatingModeFlags::SINGLE_CONVERSION; }
    impl OperatingMode for OperatingModePowerDown { const OPERATING_MODE: OperatingModeFlags = OperatingModeFlags::POWER_DOWN; }
}
