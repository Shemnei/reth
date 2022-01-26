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

	pub type IntWidth = i64;
	pub type IntWidthU = i64;
	pub type FloatWidth = f64;
	pub type Width = IntWidth;

	pub type Word = i32;
	pub type HalfWord = i16;
	pub type DoulbeWord = i64;
	pub type QuadWord = i128;

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

		read!(read_u8: u8: le);

		read!(read_u16_be: u16: be);
		read!(read_u16_le: u16: le);

		read!(read_u32_be: u32: be);
		read!(read_u32_le: u32: le);

		read!(read_u64_be: u64: be);
		read!(read_u64_le: u64: le);

		read!(read_u128_be: u128: be);
		read!(read_u128_le: u128: le);

		write!(write_u8: u8: le);

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
	use crate::tra::Trap;

	#[derive(Default, Debug)]
	pub struct Memory(pub Vec<u8>);

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

	impl Addressable for MemoryManagementUnit {
		type Address = Address;
		type Error = Trap;

		fn len(&self) -> usize {
			self.memory.len()
		}

		fn read(
			&mut self,
			addr: Self::Address,
			data: &mut [u8],
		) -> Result<(), Self::Error> {
			Ok(self.memory.read(addr, data).unwrap())
		}

		fn write(
			&mut self,
			addr: Self::Address,
			data: &[u8],
		) -> Result<(), Self::Error> {
			Ok(self.memory.write(addr, data).unwrap())
		}
	}
}

pub mod reg {
	use crate::shared::{FloatWidth, IntWidth};

	macro_rules! regs {
		(
			$regs:ident {
				$(
					$( #[doc = $doc:literal] )*
					Reg( $idx:literal => $ident:ident, name = $name:literal, desc = $desc:literal )
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
					$ident = $idx,
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

			impl std::convert::TryFrom<u8> for $regs {
				type Error = ();

				fn try_from(value: u8) -> Result<Self, Self::Error> {
					match value {
						$(
							$idx => Ok(Self::$ident),
						)+
						_ => Err(()),
					}
				}
			}
		};
	}

	regs! {
		IntReg {
			Reg( 0 => x0, name = "Zero", desc = "Always zero"),
			Reg( 1 => x1, name = "ra", desc = "Return address"),
			Reg( 2 => x2, name = "sp", desc = "Stack pointer"),
			Reg( 3 => x3, name = "gp", desc = "Global pointer"),
			Reg( 4 => x4, name = "tp", desc = "Thread pointer"),
			Reg( 5 => x5, name = "t0", desc = "Temporary / alternate return address"),
			Reg( 6 => x6, name = "t1", desc = "Temporary"),
			Reg( 7 => x7, name = "t2", desc = "Temporary"),
			Reg( 8 => x8, name = "s0", desc = "Saved register / frame pointer"),
			Reg( 9 => x9, name = "s1", desc = "Saved register"),
			Reg(10 => x10, name = "a0", desc = "Function argument / return value"),
			Reg(11 => x11, name = "a1", desc = "Function argument"),
			Reg(12 => x12, name = "a2", desc = "Function argument"),
			Reg(13 => x13, name = "a3", desc = "Function argument"),
			Reg(14 => x14, name = "a4", desc = "Function argument"),
			Reg(15 => x15, name = "a5", desc = "Function argument"),
			Reg(16 => x16, name = "a6", desc = "Function argument"),
			Reg(17 => x17, name = "a7", desc = "Function argument"),
			Reg(18 => x18, name = "s2", desc = "Saved register"),
			Reg(19 => x19, name = "s3", desc = "Saved register"),
			Reg(20 => x20, name = "s4", desc = "Saved register"),
			Reg(21 => x21, name = "s5", desc = "Saved register"),
			Reg(22 => x22, name = "s6", desc = "Saved register"),
			Reg(23 => x23, name = "s7", desc = "Saved register"),
			Reg(24 => x24, name = "s8", desc = "Saved register"),
			Reg(25 => x25, name = "s9", desc = "Saved register"),
			Reg(26 => x26, name = "s10", desc = "Saved register"),
			Reg(27 => x27, name = "s11", desc = "Saved register"),
			Reg(28 => x28, name = "t3", desc = "Temporary"),
			Reg(29 => x29, name = "t4", desc = "Temporary"),
			Reg(30 => x30, name = "t5", desc = "Temporary"),
			Reg(31 => x31, name = "t6", desc = "Temporary")
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

	impl std::ops::Index<IntReg> for IntRegisters {
		type Output = IntWidth;

		fn index(&self, index: IntReg) -> &Self::Output {
			&self.0[index as usize]
		}
	}

	impl std::ops::IndexMut<IntReg> for IntRegisters {
		fn index_mut(&mut self, index: IntReg) -> &mut Self::Output {
			// TODO: prevent setting of `x0`
			&mut self.0[index as usize]
		}
	}

	regs! {
		FloatReg {
			Reg( 0 => f0, name = "ft0", desc = "Floating-point temporaries"),
			Reg( 1 => f1, name = "ft1", desc = "Floating-point temporaries"),
			Reg( 2 => f2, name = "ft2", desc = "Floating-point temporaries"),
			Reg( 3 => f3, name = "ft3", desc = "Floating-point temporaries"),
			Reg( 4 => f4, name = "ft4", desc = "Floating-point temporaries"),
			Reg( 5 => f5, name = "ft5", desc = "Floating-point temporaries"),
			Reg( 6 => f6, name = "ft6", desc = "Floating-point temporaries"),
			Reg( 7 => f7, name = "ft7", desc = "Floating-point temporaries"),
			Reg( 8 => f8, name = "fs0", desc = "Floating-point saved registers"),
			Reg( 9 => f9, name = "fs1", desc = "Floating-point saved registers"),
			Reg(10 => f10, name = "fa0", desc = "Floating-point arguments/return values"),
			Reg(11 => f11, name = "fa1", desc = "Floating-point arguments/return values"),
			Reg(12 => f12, name = "fa2", desc = "Floating-point arguments/return values"),
			Reg(13 => f13, name = "fa3", desc = "Floating-point arguments/return values"),
			Reg(14 => f14, name = "fa4", desc = "Floating-point arguments/return values"),
			Reg(15 => f15, name = "fa5", desc = "Floating-point arguments/return values"),
			Reg(16 => f16, name = "fa6", desc = "Floating-point arguments/return values"),
			Reg(17 => f17, name = "fa7", desc = "Floating-point arguments/return values"),
			Reg(18 => f18, name = "fs2", desc = "Floating-point saved registers"),
			Reg(19 => f19, name = "fs3", desc = "Floating-point saved registers"),
			Reg(20 => f20, name = "fs4", desc = "Floating-point saved registers"),
			Reg(21 => f21, name = "fs5", desc = "Floating-point saved registers"),
			Reg(22 => f22, name = "fs6", desc = "Floating-point saved registers"),
			Reg(23 => f23, name = "fs7", desc = "Floating-point saved registers"),
			Reg(24 => f24, name = "fs8", desc = "Floating-point saved registers"),
			Reg(25 => f25, name = "fs9", desc = "Floating-point saved registers"),
			Reg(26 => f26, name = "fs10", desc = "Floating-point saved registers"),
			Reg(27 => f27, name = "fs11", desc = "Floating-point saved registers"),
			Reg(28 => f28, name = "ft8", desc = "Floating-point temporaries"),
			Reg(29 => f29, name = "ft9", desc = "Floating-point temporaries"),
			Reg(30 => f30, name = "ft10", desc = "Floating-point temporaries"),
			Reg(31 => f31, name = "ft11", desc = "Floating-point temporaries")
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
	use crate::adr::Addressable;
	use crate::ins::{Instruction, INSTRUCTIONS};
	use crate::mem::MemoryManagementUnit;
	use crate::reg::{FloatRegisters, IntReg, IntRegisters};
	use crate::shared::{Address, IntWidth, Word};
	use crate::tra::Trap;

	pub type Result<T, E = Trap> = std::result::Result<T, E>;

	pub const PC_STEP: Address = 4;

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
		pub pc: Address,
		pub xregs: IntRegisters,
		pub fregs: FloatRegisters,

		// Memory
		pub mmu: MemoryManagementUnit,
	}

	impl Cpu {
		pub fn tick(&mut self) {
			let inst_addr = self.pc;

			let word = match self.fetch() {
				Ok(word) => word,
				Err(trap) => {
					self.handle_trap(trap);
					return;
				}
			};

			// TODO: check instruction size (p. 8/26)

			self.step_pc(PC_STEP);

			let inst = self.decode(word).unwrap_or_else(|| {
				panic!(
					"Unknown instruction (pc: 0x{:016x}; inst: 0b{:032b}; \
					 should: {:#?})",
					// TODO: remove riscv_decode
					inst_addr,
					word,
					riscv_decode::decode(word)
				)
			});

			println!(">> Running: {}/{}", inst.extension, inst.name);

			if let Err(trap) = (inst.op)(self, word, inst_addr) {
				self.handle_trap(trap);
				// Reset `x0` to `0` (allowed through Index)
				// TODO: fix
				self.xregs[IntReg::x0] = 0;
				return;
			}

			self.mmu.tick();
		}

		fn handle_trap(&mut self, trap: Trap) {}

		fn fetch(&mut self) -> Result<u32, Trap> {
			match self.mmu.read_u32_le(self.pc) {
				Ok(word) => Ok(word),
				Err(err) => {
					self.step_pc(PC_STEP);
					Err(err)
				}
			}
		}

		fn step_pc(&mut self, step: Address) {
			self.pc = self.pc.wrapping_add(step);
		}

		fn decode(&mut self, word: u32) -> Option<&Instruction> {
			// TODO: cache

			for inst in &INSTRUCTIONS {
				if word & inst.mask == inst.reqd {
					return Some(inst);
				}
			}

			None
		}
	}
}
