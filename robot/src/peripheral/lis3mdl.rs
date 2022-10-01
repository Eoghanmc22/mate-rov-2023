use crate::peripheral::spi::Device;

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

// TODO who am i

pub trait WriteableRegister {
    fn write(&self, dest: &mut impl Device);
}

//TODO is this needed?
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

// todo visibility
pub mod crtl_reg_1 {
    use std::marker::PhantomData;
    use bitflags::bitflags;
    use crate::peripheral::lis3mdl::{Field, WriteableRegister};
    use crate::peripheral::spi::Device;

    // FIXME prob need phantom data to compile
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
            const ENABLE  = 0b0000_0001;
            const DISABLE = 0b0000_0000;
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
            const ENABLE  = 0b0000_0001;
            const DISABLE = 0b0000_0000;
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
            const ENABLE  = 0b0000_0001;
            const DISABLE = 0b0000_0000;
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

