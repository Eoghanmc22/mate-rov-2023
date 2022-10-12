use crate::define_peripheral;

define_peripheral!(
    Lis3mdl:
        reg CtrlReg1, addrs=0x20
            field Temperature, default=Disable, shift=7
                flag Enable, 0b1
                flag Disable, 0b0
            field PerformanceXY, default=Low, shift=5
                flag Low, 0b00
                flag Medium, 0b01
                flag High, 0b10
                flag UltraHigh, 0b11
            field OutputDataRate, default=_10_0, shift=2
                flag _0_625, 0b000
                flag _1_25, 0b001
                flag _2_5, 0b010
                flag _5_0, 0b011
                flag _10_0, 0b100
                flag _20_0, 0b101
                flag _40_0, 0b110
                flag _80_0, 0b111
            field FastOdr, default=Disable, shift=1
                flag Enable, 0b1
                flag Disable, 0b0
            field SelfTest, default=Disable, shift=0
                flag Enable, 0b1
                flag Disable, 0b0
        reg CtrlReg2, addrs=0x21
            field Scale, default=_4Gauss, shift=5 const SENSITIVITY: f64;
                flag _4Gauss, 0b00 const SENSITIVITY: f64 = 1.0 / 6842.0;
                flag _8Gauss, 0b01 const SENSITIVITY: f64 = 1.0 / 3421.0;
                flag _12Gauss, 0b10 const SENSITIVITY: f64 = 1.0 / 2281.0;
                flag _16Gauss, 0b11 const SENSITIVITY: f64 = 1.0 / 1711.0;
            field Reboot, default=NormalMode, shift=3
                flag Active, 0b1
                flag NormalMode, 0b0
            field SoftReset, default=NormalMode, shift=2
                flag Active, 0b1
                flag NormalMode, 0b0
        reg CtrlReg3, addrs=0x22
            field LowPower, default=Disable, shift=5
                flag Enable, 0b1
                flag Disable, 0b0
            field SpiMode, default=_4Wire, shift=2
                flag _4Wire, 0b0
                flag _3Wire, 0b1
            field OperatingMode, default=PowerDown, shift=0
                flag ContinuousConversion, 0b00
                flag SingleConversion, 0b01
                flag PowerDown, 0b11
        reg CtrlReg4, addrs=0x23
            field PerformanceZ, default=Low, shift=2
                flag Low, 0b00
                flag Medium, 0b01
                flag High, 0b10
                flag UltraHigh, 0b11
            field Endianness, default=Big, shift=1
                flag Big, 0b0
                flag Little, 0b1
        reg CtrlReg5, addrs=0x24
            field FastRead, default=Disable, shift=7
                flag Enable, 0b1
                flag Disable, 0b0
            field BlockDataUpdate, default=Disable, shift=6
                flag Enable, 0b1
                flag Disable, 0b0
);

//todo who_am_i status and out

#[cfg(test)]
mod tests {
    use std::mem;
    use crate::peripheral::devices::lis3mdl::ctrl_reg1::{CtrlReg1, FastOdrEnable, OutputDataRate_2_5, PerformanceXYMedium, SelfTestEnable, TemperatureEnable};
    use crate::peripheral::devices::lis3mdl::ctrl_reg2::{CtrlReg2, RebootActive, Scale_12Gauss, SoftResetActive};
    use crate::peripheral::devices::lis3mdl::ctrl_reg3::{CtrlReg3, LowPowerEnable, OperatingModeSingleConversion, SpiMode_3Wire};
    use crate::peripheral::devices::lis3mdl::ctrl_reg4::{CtrlReg4, EndiannessLittle, PerformanceZHigh};
    use crate::peripheral::devices::lis3mdl::ctrl_reg5::{BlockDataUpdateEnable, CtrlReg5, FastReadEnable};
    use crate::peripheral::devices::lis3mdl::{Lis3mdl, Lis3mdlRegisters};
    use crate::peripheral::WriteableRegister;

    #[test]
    fn size() {
        assert_eq!(mem::size_of::<Lis3mdl>(), 0);
    }

    #[test]
    fn default() {
        type Chip = Lis3mdl;
        assert_eq!(<Chip as Lis3mdlRegisters>::CtrlReg1::ADDRESS, 0x20);
        assert_eq!(<Chip as Lis3mdlRegisters>::CtrlReg1::BYTE, 0b0001_0000);

        assert_eq!(<Chip as Lis3mdlRegisters>::CtrlReg2::ADDRESS, 0x21);
        assert_eq!(<Chip as Lis3mdlRegisters>::CtrlReg2::BYTE, 0b0000_0000);

        assert_eq!(<Chip as Lis3mdlRegisters>::CtrlReg3::ADDRESS, 0x22);
        assert_eq!(<Chip as Lis3mdlRegisters>::CtrlReg3::BYTE, 0b0000_0011);

        assert_eq!(<Chip as Lis3mdlRegisters>::CtrlReg4::ADDRESS, 0x23);
        assert_eq!(<Chip as Lis3mdlRegisters>::CtrlReg4::BYTE, 0b0000_0000);

        assert_eq!(<Chip as Lis3mdlRegisters>::CtrlReg5::ADDRESS, 0x24);
        assert_eq!(<Chip as Lis3mdlRegisters>::CtrlReg5::BYTE, 0b0000_0000);
    }

    #[test]
    fn states() {
        assert_eq!(<CtrlReg1<TemperatureEnable, PerformanceXYMedium, OutputDataRate_2_5, FastOdrEnable, SelfTestEnable>>::BYTE, 0b1010_1011);

        assert_eq!(<CtrlReg2<Scale_12Gauss, RebootActive, SoftResetActive>>::BYTE, 0b0100_1100);

        assert_eq!(<CtrlReg3<LowPowerEnable, SpiMode_3Wire, OperatingModeSingleConversion>>::BYTE, 0b0010_0101);

        assert_eq!(<CtrlReg4<PerformanceZHigh, EndiannessLittle>>::BYTE, 0b0000_1010);

        assert_eq!(<CtrlReg5<FastReadEnable, BlockDataUpdateEnable>>::BYTE, 0b1100_0000);
    }
}
