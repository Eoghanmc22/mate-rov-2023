//pub mod lis3mdl;
//pub mod lsm6dsl;
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

pub trait Register {
    const ADDRESS: u8;
    // TODO: Make Data part of Register

    fn setup(_dev: &mut impl Device) -> anyhow::Result<()> {
        Ok(())
    }
}

pub trait WriteableRegister: Register {
    const BYTE: u8;

    fn write(dev: &mut impl Device) -> anyhow::Result<()> {
        dev.write_byte(Self::ADDRESS, Self::BYTE)
    }
}

pub trait ReadableRegister: Register {
    type Data;

    fn read(dev: &mut impl Device) -> anyhow::Result<Self::Data>;
}

pub trait FixedRegister where Self: ReadableRegister<Data=u8> {
    const BYTE: u8;

    fn check(dev: &mut impl Device) -> anyhow::Result<bool> {
        Self::read(dev).map(|read| read == Self::BYTE)
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
    ($chip_name:ident: $(
        reg $register_name:ident, addrs=$register_addrs:literal $(, doc=$reg_purpose:literal)?, type=$(config $(field $field_name:ident, default=$field_default:ident, shift=$field_shift:literal $(, doc=$field_purpose:literal)? $(const $field_data_name:ident: $field_data_type:ty;)* $(flag $flag_name:ident, $flag_val:literal $(, doc=$flag_purpose:literal)? $(const $flag_data_name:ident: $flag_data_type:ty = $flag_data_expr:expr;)*)+)+)? $(val, data=$reg_type:ty;)? $(raw, flags=($($raw_gen:ident),*);)? $(fixed, val=$fixed_val:literal;)?)+) => {
        paste::paste! {
            pub mod [<$chip_name:snake:lower>] {
                pub struct $chip_name<$($($([<$field_name _>]: [<$register_name:snake:lower>]::$field_name = [<$register_name:snake:lower>]::[<$field_name $field_default>],)+)?)+>(std::marker::PhantomData<($($($([<$field_name _>],)+)?)+)>);

                impl<$($($([<$field_name _>]: [<$register_name:snake:lower>]::$field_name,)+)?)+> $chip_name<$($($([<$field_name _>],)+)?)+> {
                     pub fn setup(dev: &mut impl $crate::peripheral::Device) -> anyhow::Result<()> {
                        $(
                            <<Self as [<$chip_name Registers>]>::$register_name as $crate::peripheral::Register>::setup(dev)?;
                        )+

                        Ok(())
                    }
                }

                pub trait [<$chip_name Registers>] {
                    $(
                        type $register_name: $crate::peripheral::Register;
                    )+
                }

                impl<$($($([<$field_name _>]: [<$register_name:snake:lower>]::$field_name,)+)?)+> [<$chip_name Registers>] for $chip_name<$($($([<$field_name _>],)+)?)+> {
                    $(
                        type $register_name = [<$register_name:snake:lower>]::$register_name<$($([<$field_name _>],)+)? $($([<$raw_gen _>],)*)?>;
                    )+
                }

                $($($(
                    $(#[doc=$field_purpose])?
                    pub trait $field_name {
                        const [<$field_name:snake:upper>]: [<$register_name:snake:lower>]::[<$field_name Flags>];
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
                        $(#[doc=$flag_purpose])?
                        pub struct [<$field_name $flag_name>];

                        impl $field_name for [<$field_name $flag_name>] {
                            const [<$field_name:snake:upper>]: [<$field_name Flags>] = [<$field_name Flags>]::$flag_name;
                            $(
                                const $flag_data_name: $flag_data_type = $flag_data_expr;
                            )*
                        }
                    )+
                )+)?)+

                $(
                    pub mod [<$register_name:snake:lower>] {
                        pub use super::*;
                        use super::super::*;
                        $(#[doc=$reg_purpose])?
                        pub struct $register_name<$($([<$field_name _>]: $field_name = [<$field_name $field_default>],)+)? $($([<$raw_gen _>]: $raw_gen,)*)?>(std::marker::PhantomData<($($([<$field_name _>],)+)? $($([<$raw_gen _>],)*)?)>);

                        $(
                            impl<$([<$field_name _>]: $field_name,)+> $crate::peripheral::Register for $register_name<$([<$field_name _>],)+>  {
                                const ADDRESS: u8 = $register_addrs;

                                fn setup(dev: &mut impl $crate::peripheral::Device) -> anyhow::Result<()> {
                                    <Self as $crate::peripheral::WriteableRegister>::write(dev)
                                }
                            }

                            impl<$([<$field_name _>]: $field_name,)+> $crate::peripheral::WriteableRegister for $register_name<$([<$field_name _>],)+>  {
                                const BYTE: u8 = $crate::fields_to_byte!($([<$field_name _>],)+);
                            }
                        )?

                        $(
                            impl $crate::peripheral::Register for $register_name {
                                const ADDRESS: u8 = $register_addrs;
                            }

                            impl $crate::peripheral::ReadableRegister for $register_name  {
                                type Data = $reg_type;
                                fn read(dev: &mut impl $crate::peripheral::Device) -> anyhow::Result<Self::Data> {
                                    Ok(dev.read_byte(<Self as $crate::peripheral::Register>::ADDRESS)?.into())
                                }
                            }
                        )?

                        $(
                            impl<$([<$raw_gen _>]: $raw_gen,)*> $crate::peripheral::Register for $register_name<$([<$raw_gen _>],)*> {
                                const ADDRESS: u8 = $register_addrs;
                            }
                        )?

                        $(
                            impl $crate::peripheral::Register for $register_name {
                                const ADDRESS: u8 = $register_addrs;
                            }

                            impl $crate::peripheral::ReadableRegister for $register_name  {
                                type Data = u8;
                                fn read(dev: &mut impl $crate::peripheral::Device) -> anyhow::Result<Self::Data> {
                                    Ok(dev.read_byte(<Self as $crate::peripheral::Register>::ADDRESS)?.into())
                                }
                            }

                            impl $crate::peripheral::FixedRegister for $register_name  {
                                const BYTE: u8 = $fixed_val;
                            }
                        )?
                    }
                )+
            }
        }
    };
}
