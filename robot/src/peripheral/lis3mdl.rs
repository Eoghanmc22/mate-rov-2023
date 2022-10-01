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
    const ADDRESS: u8;
    const BYTE: u8;

    fn write(&self, dest: &mut impl Device) -> anyhow::Result<()> {
        dest.write_byte(Self::ADDRESS, Self::BYTE)
    }
}

trait Field<T> {
    const FIELD: T;
    const SHIFT: usize;
}

macro_rules! fields_to_byte {
    ($( $field:ty ),+) => {
        $( (<$field as Field<_>>::FIELD).bits() << <$field as Field<_>>::SHIFT | )* 0
    };
}

//TODO implement useful stuff lol
pub struct Lis3mdl {
    // TODO who_am_i
    ctrl_reg_1: ctrl_reg_1::CtrlReg1,
    ctrl_reg_2: ctrl_reg_2::CtrlReg2,
    ctrl_reg_3: ctrl_reg_3::CtrlReg3,
    ctrl_reg_4: ctrl_reg_4::CtrlReg4,
    ctrl_reg_5: ctrl_reg_5::CtrlReg5,
    // TODO status_reg

    // TODO out block
    // TODO interrupts
}

pub mod ctrl_reg_1 {
    use std::marker::PhantomData;
    use bitflags::bitflags;
    use crate::peripheral::lis3mdl::{Field, WriteableRegister};

    #[derive(Copy, Clone, Default)]
    pub struct CtrlReg1<Temperature = TemperatureDisable, PerformanceXY = PerformanceLowXY, OutputDataRate = OutputDataRate10_0, FastOdr = FastOdrDisable, SelfTest = SelfTestDisable>(PhantomData<(Temperature, PerformanceXY, OutputDataRate, FastOdr, SelfTest)>);

    impl<Temperature_: Temperature, PerformanceXY_: PerformanceXY, OutputDataRate_: OutputDataRate, FastOdr_: FastOdr, SelfTest_: SelfTest> WriteableRegister for CtrlReg1<Temperature_, PerformanceXY_, OutputDataRate_, FastOdr_, SelfTest_>  {
        const ADDRESS: u8 = 0x20;
        const BYTE: u8 = fields_to_byte!(Temperature_, PerformanceXY_, OutputDataRate_, FastOdr_, SelfTest_);
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
        pub struct PerformanceXYFlags: u8 {
            const LOW_PERFORMANCE        = 0b00;
            const MEDIUM_PERFORMANCE     = 0b01;
            const HIGH_PERFORMANCE       = 0b10;
            const ULTRA_HIGH_PERFORMANCE = 0b11;
        }
    }

    pub trait PerformanceXY {
        const PERFORMANCE_XY: PerformanceXYFlags;
        const SHIFT: usize = 5;
    }

    impl<T> Field<PerformanceXYFlags> for T where T: PerformanceXY {
        const FIELD: PerformanceXYFlags = <T as PerformanceXY>::PERFORMANCE_XY;
        const SHIFT: usize = <T as PerformanceXY>::SHIFT;
    }

    pub struct PerformanceLowXY;
    pub struct PerformanceMediumXY;
    pub struct PerformanceHighXY;
    pub struct PerformanceUltraHighXY;
    impl PerformanceXY for PerformanceLowXY { const PERFORMANCE_XY: PerformanceXYFlags = PerformanceXYFlags::LOW_PERFORMANCE; }
    impl PerformanceXY for PerformanceMediumXY { const PERFORMANCE_XY: PerformanceXYFlags = PerformanceXYFlags::MEDIUM_PERFORMANCE; }
    impl PerformanceXY for PerformanceHighXY { const PERFORMANCE_XY: PerformanceXYFlags = PerformanceXYFlags::HIGH_PERFORMANCE; }
    impl PerformanceXY for PerformanceUltraHighXY { const PERFORMANCE_XY: PerformanceXYFlags = PerformanceXYFlags::ULTRA_HIGH_PERFORMANCE; }


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

    #[derive(Copy, Clone, Default)]
    pub struct CtrlReg2<Scale = Scale4Gauss, Reboot = RebootNormalMode, SoftReset = SoftResetNormalMode>(PhantomData<(Scale, Reboot, SoftReset)>);

    impl<Scale_: Scale, Reboot_: Reboot, SoftReset_: SoftReset> WriteableRegister for CtrlReg2<Scale_, Reboot_, SoftReset_>  {
        const ADDRESS: u8 = 0x21;
        const BYTE: u8 = fields_to_byte!(Scale_, Reboot_, SoftReset_);
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
    impl SoftReset for SoftResetNormalMode { const SOFT_RESET: SoftResetFlags = SoftResetFlags::NORMAL_MODE; }
    impl SoftReset for SoftResetRegisters { const SOFT_RESET: SoftResetFlags = SoftResetFlags::RESET; }
}

pub mod ctrl_reg_3 {
    use std::marker::PhantomData;
    use bitflags::bitflags;
    use crate::peripheral::lis3mdl::{Field, WriteableRegister};

    #[derive(Copy, Clone, Default)]
    pub struct CtrlReg3<LowPower = LowPowerDisable, SpiMode = SpiMode4Wire, OperatingMode = OperatingModePowerDown>(PhantomData<(LowPower, SpiMode, OperatingMode)>);

    impl<LowPower_: LowPower, SpiMode_: SpiMode, OperatingMode_: OperatingMode> WriteableRegister for CtrlReg3<LowPower_, SpiMode_, OperatingMode_>  {
        const ADDRESS: u8 = 0x22;
        const BYTE: u8 = fields_to_byte!(LowPower_, SpiMode_, OperatingMode_);
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

pub mod ctrl_reg_4 {
    use std::marker::PhantomData;
    use bitflags::bitflags;
    use crate::peripheral::lis3mdl::{Field, WriteableRegister};

    #[derive(Copy, Clone, Default)]
    pub struct CtrlReg4<PerformanceZ = PerformanceLowZ, Endianness = EndiannessBig>(PhantomData<(PerformanceZ, Endianness)>);

    impl<PerformanceZ_: PerformanceZ, Endianness_: Endianness> WriteableRegister for CtrlReg4<PerformanceZ_, Endianness_>  {
        const ADDRESS: u8 = 0x23;
        const BYTE: u8 = fields_to_byte!(PerformanceZ_, Endianness_);
    }


    bitflags! {
        pub struct PerformanceZFlags: u8 {
            const LOW_PERFORMANCE        = 0b00;
            const MEDIUM_PERFORMANCE     = 0b01;
            const HIGH_PERFORMANCE       = 0b10;
            const ULTRA_HIGH_PERFORMANCE = 0b11;
        }
    }

    pub trait PerformanceZ {
        const PERFORMANCE_Z: PerformanceZFlags;
        const SHIFT: usize = 2;
    }

    impl<T> Field<PerformanceZFlags> for T where T: PerformanceZ {
        const FIELD: PerformanceZFlags = <T as PerformanceZ>::PERFORMANCE_Z;
        const SHIFT: usize = <T as PerformanceZ>::SHIFT;
    }

    pub struct PerformanceLowZ;
    pub struct PerformanceMediumZ;
    pub struct PerformanceHighZ;
    pub struct PerformanceUltraHighZ;
    impl PerformanceZ for PerformanceLowZ { const PERFORMANCE_Z: PerformanceZFlags = PerformanceZFlags::LOW_PERFORMANCE; }
    impl PerformanceZ for PerformanceMediumZ { const PERFORMANCE_Z: PerformanceZFlags = PerformanceZFlags::MEDIUM_PERFORMANCE; }
    impl PerformanceZ for PerformanceHighZ { const PERFORMANCE_Z: PerformanceZFlags = PerformanceZFlags::HIGH_PERFORMANCE; }
    impl PerformanceZ for PerformanceUltraHighZ { const PERFORMANCE_Z: PerformanceZFlags = PerformanceZFlags::ULTRA_HIGH_PERFORMANCE; }


    bitflags! {
        pub struct EndiannessFlags: u8 {
            const BIG_ENDIAN    = 0b0;
            const LITTLE_ENDIAN = 0b1;
        }
    }

    pub trait Endianness {
        const ENDIANNESS: EndiannessFlags;
        const SHIFT: usize = 1;
    }

    impl<T> Field<EndiannessFlags> for T where T: Endianness {
        const FIELD: EndiannessFlags = <T as Endianness>::ENDIANNESS;
        const SHIFT: usize = <T as Endianness>::SHIFT;
    }

    pub struct EndiannessBig;
    pub struct EndiannessLittle;
    impl Endianness for EndiannessBig { const ENDIANNESS: EndiannessFlags = EndiannessFlags::BIG_ENDIAN; }
    impl Endianness for EndiannessLittle { const ENDIANNESS: EndiannessFlags = EndiannessFlags::LITTLE_ENDIAN; }
}

pub mod ctrl_reg_5 {
    use std::marker::PhantomData;
    use bitflags::bitflags;
    use crate::peripheral::lis3mdl::{Field, WriteableRegister};

    #[derive(Copy, Clone, Default)]
    pub struct CtrlReg5<FastRead = FastReadDisable, BlockDataUpdate = BlockDataUpdateDisable>(PhantomData<(FastRead, BlockDataUpdate)>);

    impl<FastRead_: FastRead, BlockDataUpdate_: BlockDataUpdate> WriteableRegister for CtrlReg5<FastRead_, BlockDataUpdate_>  {
        const ADDRESS: u8 = 0x24;
        const BYTE: u8 = fields_to_byte!(FastRead_, BlockDataUpdate_);
    }


    bitflags! {
        pub struct FastReadFlags: u8 {
            const ENABLE  = 0b1;
            const DISABLE = 0b0;
        }
    }

    pub trait FastRead {
        const FAST_READ: FastReadFlags;
        const SHIFT: usize = 7;
    }

    impl<T> Field<FastReadFlags> for T where T: FastRead {
        const FIELD: FastReadFlags = <T as FastRead>::FAST_READ;
        const SHIFT: usize = <T as FastRead>::SHIFT;
    }

    pub struct FastReadEnable;
    pub struct FastReadDisable;
    impl FastRead for FastReadEnable { const FAST_READ: FastReadFlags = FastReadFlags::ENABLE; }
    impl FastRead for FastReadDisable { const FAST_READ: FastReadFlags = FastReadFlags::DISABLE; }


    bitflags! {
        pub struct BlockDataUpdateFlags: u8 {
            const ENABLE  = 0b1;
            const DISABLE = 0b0;
        }
    }

    pub trait BlockDataUpdate {
        const BLOCK_DATA_UPDATE: BlockDataUpdateFlags;
        const SHIFT: usize = 6;
    }

    impl<T> Field<BlockDataUpdateFlags> for T where T: BlockDataUpdate {
        const FIELD: BlockDataUpdateFlags = <T as BlockDataUpdate>::BLOCK_DATA_UPDATE;
        const SHIFT: usize = <T as BlockDataUpdate>::SHIFT;
    }

    pub struct BlockDataUpdateEnable;
    pub struct BlockDataUpdateDisable;
    impl BlockDataUpdate for BlockDataUpdateEnable { const BLOCK_DATA_UPDATE: BlockDataUpdateFlags = BlockDataUpdateFlags::ENABLE; }
    impl BlockDataUpdate for BlockDataUpdateDisable { const BLOCK_DATA_UPDATE: BlockDataUpdateFlags = BlockDataUpdateFlags::DISABLE; }
}

#[cfg(test)]
mod tests {
    use crate::peripheral::lis3mdl::ctrl_reg_1::{CtrlReg1, FastOdrEnable, OutputDataRate2_5, PerformanceMediumXY, SelfTestEnable, TemperatureEnable};
    use crate::peripheral::lis3mdl::ctrl_reg_2::{CtrlReg2, RebootMemory, Scale, Scale12Gauss, SoftResetRegisters};
    use crate::peripheral::lis3mdl::ctrl_reg_3::{CtrlReg3, LowPowerEnable, OperatingModeSingleConversion, SpiMode3Wire};
    use crate::peripheral::lis3mdl::ctrl_reg_4::{CtrlReg4, EndiannessLittle, PerformanceHighZ};
    use crate::peripheral::lis3mdl::ctrl_reg_5::{BlockDataUpdateEnable, CtrlReg5, FastReadEnable};
    use crate::peripheral::lis3mdl::WriteableRegister;

    #[test]
    fn default() {
        assert_eq!(<CtrlReg1>::ADDRESS, 0x20);
        assert_eq!(<CtrlReg1>::BYTE, 0b0001_0000);

        assert_eq!(<CtrlReg2>::ADDRESS, 0x21);
        assert_eq!(<CtrlReg2>::BYTE, 0b0000_0000);

        assert_eq!(<CtrlReg3>::ADDRESS, 0x22);
        assert_eq!(<CtrlReg3>::BYTE, 0b0000_0011);

        assert_eq!(<CtrlReg4>::ADDRESS, 0x23);
        assert_eq!(<CtrlReg4>::BYTE, 0b0000_0000);

        assert_eq!(<CtrlReg5>::ADDRESS, 0x24);
        assert_eq!(<CtrlReg5>::BYTE, 0b0000_0000);
    }

    #[test]
    fn states() {
        assert_eq!(<CtrlReg1<TemperatureEnable, PerformanceMediumXY, OutputDataRate2_5, FastOdrEnable, SelfTestEnable>>::BYTE, 0b1010_1011);

        assert_eq!(<CtrlReg2<Scale12Gauss, RebootMemory, SoftResetRegisters>>::BYTE, 0b0100_1100);

        assert_eq!(<CtrlReg3<LowPowerEnable, SpiMode3Wire, OperatingModeSingleConversion>>::BYTE, 0b0010_0101);

        assert_eq!(<CtrlReg4<PerformanceHighZ, EndiannessLittle>>::BYTE, 0b0000_1010);

        assert_eq!(<CtrlReg5<FastReadEnable, BlockDataUpdateEnable>>::BYTE, 0b1100_0000);
    }
}
