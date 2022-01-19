#![allow(dead_code)]
#[cfg(not(target_pointer_width = "64"))]
compile_error!("This emulator requires a 64-bit system");

// References:
// - https://github.com/riscv/riscv-isa-manual

pub mod ins;

pub mod shared {
	pub const XLEN: usize = 64;

	/// IALIGN (either 16/32 for instruction address alignment)
	/// ILEN   (max. instruction length in bits)

	pub type IntWidth = u64;
	pub type FloatWidth = f64;
	pub type Width = IntWidth;

	pub type Word = u32;
	pub type HalfWord = u16;
	pub type DoulbeWord = u64;
	pub type QuadWord = u128;

	pub type Address = u64;

	#[test]
	fn assert_twos_complement() {
		assert_eq!(unsafe { core::mem::transmute::<i8, u8>(-128_i8) }, 128_u8);

		assert_eq!(unsafe { core::mem::transmute::<u8, i8>(255_u8) }, -1_i8);
	}

	#[test]
	fn assert_sign_extend() {
		assert_eq!(
			unsafe { core::mem::transmute::<i16, u16>(1_i8 as i16) },
			1_u16
		);

		assert_eq!(
			unsafe { core::mem::transmute::<i16, u16>(-128_i8 as i16) },
			0xff80_u16
		);
	}
}

pub mod adr {
	macro_rules! read {
		( $fn:ident : $size:ty : be ) => {
			fn $fn(
				&mut self,
				addr: Self::Address,
			) -> Result<$size, Self::Error> {
				let mut buf = [0u8; std::mem::size_of::<$size>()];
				self.read(addr, &mut buf)?;
				Ok(<$size>::from_be_bytes(buf))
			}
		};
		( $fn:ident : $size:ty : le ) => {
			fn $fn(
				&mut self,
				addr: Self::Address,
			) -> Result<$size, Self::Error> {
				let mut buf = [0u8; std::mem::size_of::<$size>()];
				self.read(addr, &mut buf)?;
				Ok(<$size>::from_le_bytes(buf))
			}
		};
	}

	macro_rules! write {
		( $fn:ident : $size:ty : be ) => {
			fn $fn(
				&mut self,
				addr: Self::Address,
				value: $size,
			) -> Result<(), Self::Error> {
				let buf = <$size>::to_be_bytes(value);
				self.write(addr, &buf)
			}
		};
		( $fn:ident : $size:ty : le ) => {
			fn $fn(
				&mut self,
				addr: Self::Address,
				value: $size,
			) -> Result<(), Self::Error> {
				let buf = <$size>::to_le_bytes(value);
				self.write(addr, &buf)
			}
		};
	}

	#[allow(clippy::len_without_is_empty)]
	pub trait Addressable {
		type Address;
		type Error;

		fn len(&self) -> usize;

		// TODO: impl is_empty? does not realy make sense

		fn read(
			&mut self,
			addr: Self::Address,
			data: &mut [u8],
		) -> Result<(), Self::Error>;

		fn write(
			&mut self,
			addr: Self::Address,
			data: &[u8],
		) -> Result<(), Self::Error>;

		read!(read_u16_be: u16: be);
		read!(read_u16_le: u16: le);

		read!(read_u32_be: u32: be);
		read!(read_u32_le: u32: le);

		read!(read_u64_be: u64: be);
		read!(read_u64_le: u64: le);

		read!(read_u128_be: u128: be);
		read!(read_u128_le: u128: le);

		write!(write_u16_be: u16: be);
		write!(write_u16_le: u16: le);

		write!(write_u32_be: u32: be);
		write!(write_u32_le: u32: le);

		write!(write_u64_be: u64: be);
		write!(write_u64_le: u64: le);

		write!(write_u128_be: u128: be);
		write!(write_u128_le: u128: le);
	}
}

pub mod tra {
	// # Exceptions
	//
	// Unusual condition at runtime
	//
	// # Interrupt
	//
	// External asynchronous event that may cause transfer of control.
	//
	// # Trap
	//
	// Trap handle which handles exception or interrupt.
	//
	// Effects:
	// - Contained Trap
	// > Visible and handled by software
	//
	// - Requested Trap
	// > Synchronous exception explicitly called software
	//
	// - Invisible Trap
	// > Handled transparently by execution env
	//
	// - Fatal Trap
	// > Causes execution env to terminate

	pub struct Trap;
}

pub mod mem {
	use crate::adr::Addressable;
	use crate::shared::Address;

	#[derive(Default, Debug)]
	pub struct Memory(Vec<u8>);

	impl Addressable for Memory {
		type Address = Address;
		type Error = ();

		fn len(&self) -> usize {
			self.0.len()
		}

		fn read(
			&mut self,
			addr: Self::Address,
			data: &mut [u8],
		) -> Result<(), Self::Error> {
			let start = addr as usize;
			let end = start + data.len();

			data.copy_from_slice(&self.0[start..end]);
			Ok(())
		}

		fn write(
			&mut self,
			addr: Self::Address,
			data: &[u8],
		) -> Result<(), Self::Error> {
			let start = addr as usize;
			let end = start + data.len();

			// TODO: resize if neccessary?
			(&mut self.0[start..end]).copy_from_slice(data);
			Ok(())
		}
	}

	#[derive(Default, Debug)]
	pub struct MemoryManagementUnit {
		pub memory: Memory,
	}

	impl MemoryManagementUnit {
		pub fn tick(&mut self) {}
	}
}

pub mod reg {
	use crate::shared::{FloatWidth, IntWidth};

	macro_rules! regs {
		(
			$regs:ident {
				$(
					$( #[doc = $doc:literal] )*
					Reg( $ident:ident, name = $name:literal, desc = $desc:literal )
				),+
			}
		) => {
			#[allow(non_camel_case_types)]
			#[repr(usize)]
			#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
			pub enum $regs {
				$(
					#[doc = "Register `"]
					#[doc = stringify!($ident)]
					#[doc = "` / `"]
					#[doc = $name]
					#[doc = "`: "]
					#[doc = $desc]
					$ident,
				)+
			}

			impl $regs {
				pub const fn name(&self) -> &'static str {
					match self {
						$(
							Self::$ident => $name,
						)+
					}
				}

				pub const fn description(&self) -> &'static str {
					match self {
						$(
							Self::$ident => $desc,
						)+
					}
				}
			}

			impl std::convert::From<$regs> for usize {
				fn from(value: $regs) -> usize {
					value as usize
				}
			}
		};
	}

	regs! {
		IntReg {
			Reg(x0, name = "Zero", desc = "Always zero"),
			Reg(x1, name = "ra", desc = "Return address"),
			Reg(x2, name = "sp", desc = "Stack pointer"),
			Reg(x3, name = "gp", desc = "Global pointer"),
			Reg(x4, name = "tp", desc = "Thread pointer"),
			Reg(x5, name = "t0", desc = "Temporary / alternate return address"),
			Reg(x6, name = "t1", desc = "Temporary"),
			Reg(x7, name = "t2", desc = "Temporary"),
			Reg(x8, name = "s0", desc = "Saved register / frame pointer"),
			Reg(x9, name = "s1", desc = "Saved register"),
			Reg(x10, name = "a0", desc = "Function argument / return value"),
			Reg(x11, name = "a1", desc = "Function argument"),
			Reg(x12, name = "a2", desc = "Function argument"),
			Reg(x13, name = "a3", desc = "Function argument"),
			Reg(x14, name = "a4", desc = "Function argument"),
			Reg(x15, name = "a5", desc = "Function argument"),
			Reg(x16, name = "a6", desc = "Function argument"),
			Reg(x17, name = "a7", desc = "Function argument"),
			Reg(x18, name = "s2", desc = "Saved register"),
			Reg(x19, name = "s3", desc = "Saved register"),
			Reg(x20, name = "s4", desc = "Saved register"),
			Reg(x21, name = "s5", desc = "Saved register"),
			Reg(x22, name = "s6", desc = "Saved register"),
			Reg(x23, name = "s7", desc = "Saved register"),
			Reg(x24, name = "s8", desc = "Saved register"),
			Reg(x25, name = "s9", desc = "Saved register"),
			Reg(x26, name = "s10", desc = "Saved register"),
			Reg(x27, name = "s11", desc = "Saved register"),
			Reg(x28, name = "t3", desc = "Temporary"),
			Reg(x29, name = "t4", desc = "Temporary"),
			Reg(x30, name = "t5", desc = "Temporary"),
			Reg(x31, name = "t6", desc = "Temporary")
		}
	}

	#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
	pub struct IntRegisters([IntWidth; 32]);

	impl IntRegisters {
		pub fn get(&self, index: IntReg) -> IntWidth {
			let index: usize = index.into();

			// The `x0` register is always zero.
			if index == 0 {
				0
			} else {
				self.0[index]
			}
		}

		pub fn set(&mut self, index: IntReg, value: IntWidth) {
			let index: usize = index.into();

			// The `x0` register is always zero. Any set is voided.
			if index != 0 {
				self.0[index] = value;
			}
		}
	}

	regs! {
		FloatReg {
			Reg(f0, name = "ft0", desc = "Floating-point temporaries"),
			Reg(f1, name = "ft1", desc = "Floating-point temporaries"),
			Reg(f2, name = "ft2", desc = "Floating-point temporaries"),
			Reg(f3, name = "ft3", desc = "Floating-point temporaries"),
			Reg(f4, name = "ft4", desc = "Floating-point temporaries"),
			Reg(f5, name = "ft5", desc = "Floating-point temporaries"),
			Reg(f6, name = "ft6", desc = "Floating-point temporaries"),
			Reg(f7, name = "ft7", desc = "Floating-point temporaries"),
			Reg(f8, name = "fs0", desc = "Floating-point saved registers"),
			Reg(f9, name = "fs1", desc = "Floating-point saved registers"),
			Reg(f10, name = "fa0", desc = "Floating-point arguments/return values"),
			Reg(f11, name = "fa1", desc = "Floating-point arguments/return values"),
			Reg(f12, name = "fa2", desc = "Floating-point arguments/return values"),
			Reg(f13, name = "fa3", desc = "Floating-point arguments/return values"),
			Reg(f14, name = "fa4", desc = "Floating-point arguments/return values"),
			Reg(f15, name = "fa5", desc = "Floating-point arguments/return values"),
			Reg(f16, name = "fa6", desc = "Floating-point arguments/return values"),
			Reg(f17, name = "fa7", desc = "Floating-point arguments/return values"),
			Reg(f18, name = "fs2", desc = "Floating-point saved registers"),
			Reg(f19, name = "fs3", desc = "Floating-point saved registers"),
			Reg(f20, name = "fs4", desc = "Floating-point saved registers"),
			Reg(f21, name = "fs5", desc = "Floating-point saved registers"),
			Reg(f22, name = "fs6", desc = "Floating-point saved registers"),
			Reg(f23, name = "fs7", desc = "Floating-point saved registers"),
			Reg(f24, name = "fs8", desc = "Floating-point saved registers"),
			Reg(f25, name = "fs9", desc = "Floating-point saved registers"),
			Reg(f26, name = "fs10", desc = "Floating-point saved registers"),
			Reg(f27, name = "fs11", desc = "Floating-point saved registers"),
			Reg(f28, name = "ft8", desc = "Floating-point temporaries"),
			Reg(f29, name = "ft9", desc = "Floating-point temporaries"),
			Reg(f30, name = "ft10", desc = "Floating-point temporaries"),
			Reg(f31, name = "ft11", desc = "Floating-point temporaries")
		}
	}

	#[derive(Default, Debug, Clone, Copy, PartialEq)]
	pub struct FloatRegisters([FloatWidth; 32]);

	impl FloatRegisters {
		pub fn get(&self, index: FloatReg) -> FloatWidth {
			let index: usize = index.into();
			self.0[index]
		}

		pub fn set(&mut self, index: FloatReg, value: FloatWidth) {
			let index: usize = index.into();
			self.0[index] = value;
		}
	}
}

pub mod cpu {
	use crate::mem::MemoryManagementUnit;
	use crate::reg::{FloatRegisters, IntRegisters};
	use crate::shared::IntWidth;

	#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
	pub enum Status {
		Initializing,
		Running,
		Halted,
	}

	impl Default for Status {
		fn default() -> Self {
			Self::Initializing
		}
	}

	#[derive(Default, Debug)]
	pub struct Cpu {
		// Status
		status: Status,

		// Registers
		pub pc: IntWidth,
		pub xregs: IntRegisters,
		pub fregs: FloatRegisters,

		// Memory
		pub mmu: MemoryManagementUnit,
	}

	impl Cpu {
		pub fn tick(&mut self) {
			self.mmu.tick();
		}
	}
}
