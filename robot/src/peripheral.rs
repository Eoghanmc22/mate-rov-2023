pub mod devices;
pub mod motor;

pub trait Device {
    fn read_byte(&mut self, address: u8) -> anyhow::Result<u8> {
        let bytes = &mut [0];
        self.read(address, bytes)?;
        Ok(bytes[0])
    }
    fn write_byte(&mut self, address: u8, byte: u8) -> anyhow::Result<()> {
        self.write(address, &[byte])
    }

    fn read(&mut self, address: u8, buffer: &mut [u8]) -> anyhow::Result<()>;
    fn write(&mut self, address: u8, buffer: &[u8]) -> anyhow::Result<()>;
}

pub trait WriteableRegister {
    const ADDRESS: u8;
    const BYTE: u8;

    fn write(dev: &mut impl Device) -> anyhow::Result<()> {
        dev.write_byte(Self::ADDRESS, Self::BYTE)
    }
}

pub trait ReadableRegister {
    const ADDRESS: u8;
    type Data;

    fn read(dev: &mut impl Device) -> anyhow::Result<Self::Data>;
}

pub trait SimpleRegister {
    const ADDRESS: u8;
    type Data: From<u8>;
}

impl<T> ReadableRegister for T where T: SimpleRegister {
    const ADDRESS: u8 = <Self as SimpleRegister>::ADDRESS;
    type Data = <Self as SimpleRegister>::Data;

    fn read(dev: &mut impl Device) -> anyhow::Result<Self::Data> {
        Ok(dev.read_byte(Self::ADDRESS)?.into())
    }
}

trait Field<T> {
    const FIELD: T;
    const SHIFT: usize;
}

#[macro_export]
macro_rules! fields_to_byte {
    ($($field:ty,)+) => {
        $( (<$field as $crate::peripheral::Field<_>>::FIELD as u8) << <$field as $crate::peripheral::Field<_>>::SHIFT | )+ 0
    };
}

#[macro_export]
macro_rules! define_peripheral {
    ($chip_name:ident: $(reg $register_name:ident, addrs=$register_addrs:literal $(field $field_name:ident, default=$field_default:ident, shift=$field_shift:literal $(const $field_data_name:ident: $field_data_type:ty;)* $(flag $flag_name:ident, $flag_val:literal $(const $flag_data_name:ident: $flag_data_type:ty = $flag_data_expr:expr;)*)+)+)+) => {
        paste::paste! {
            pub mod [<$chip_name:snake:lower>] {
                // TODO debug impl using stringify!()
                pub struct $chip_name<$($([<$field_name _>]: [<$register_name:snake:lower>]::$field_name = [<$register_name:snake:lower>]::[<$field_name $field_default>],)+)+>(std::marker::PhantomData<($($([<$field_name _>],)+)+)>);

                impl<$($([<$field_name _>]: [<$register_name:snake:lower>]::$field_name,)+)+> $chip_name<$($([<$field_name _>],)+)+> {
                    pub fn setup(dev: &mut impl $crate::peripheral::Device) -> anyhow::Result<()> {
                        $(
                            <<Self as [<$chip_name Registers>]>::$register_name as $crate::peripheral::WriteableRegister>::write(dev)?;
                        )+

                        Ok(())
                    }
                }

                pub trait [<$chip_name Registers>] {
                    $(
                        type $register_name;
                    )+
                }

                impl<$($([<$field_name _>]: [<$register_name:snake:lower>]::$field_name,)+)+> [<$chip_name Registers>] for $chip_name<$($([<$field_name _>],)+)+> {
                    $(
                        type $register_name = [<$register_name:snake:lower>]::$register_name<$([<$field_name _>],)+>;
                    )+
                }

                $(
                    pub mod [<$register_name:snake:lower>] {
                        pub struct $register_name<$([<$field_name _>]: $field_name = [<$field_name $field_default>],)+>(pub(super) std::marker::PhantomData<($([<$field_name _>],)+)>);

                        impl<$([<$field_name _>]: $field_name,)+> $crate::peripheral::WriteableRegister for $register_name<$([<$field_name _>],)+>  {
                            const ADDRESS: u8 = $register_addrs;
                            const BYTE: u8 = $crate::fields_to_byte!($([<$field_name _>],)+);
                        }

                        $(
                            pub trait $field_name {
                                const [<$field_name:snake:upper>]: [<$field_name Flags>];
                                const SHIFT: usize = $field_shift;
                                $(
                                    const $field_data_name: $field_data_type;
                                )*
                            }

                            impl<T> $crate::peripheral::Field<[<$field_name Flags>]> for T where T: $field_name {
                                const FIELD: [<$field_name Flags>] = <T as $field_name>::[<$field_name:snake:upper>];
                                const SHIFT: usize = <T as $field_name>::SHIFT;
                            }

                            pub enum [<$field_name Flags>] {
                                $(
                                    $flag_name = $flag_val,
                                )+
                            }

                            $(
                                #[allow(non_camel_case_types)]
                                pub struct [<$field_name $flag_name>];

                                impl $field_name for [<$field_name $flag_name>] {
                                    const [<$field_name:snake:upper>]: [<$field_name Flags>] = [<$field_name Flags>]::$flag_name;
                                    $(
                                        const $flag_data_name: $flag_data_type = $flag_data_expr;
                                    )*
                                }
                            )+
                        )+
                    }
                )+
            }
        }
    };
}
