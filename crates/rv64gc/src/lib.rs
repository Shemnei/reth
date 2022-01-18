#![allow(dead_code)]
#[cfg(not(target_pointer_width = "64"))]
compile_error!("This emulator requires a 64-bit system");

/// References:
/// - https://github.com/riscv/riscv-isa-manual

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
		assert_eq!(format!("{:b}", -128_i8), "10000000");
	}

	#[test]
	fn assert_sign_extend() {
		assert_eq!(format!("{:016b}", 1_i8 as i16), "0000000000000001");
		assert_eq!(format!("{:016b}", -128_i8 as i16), "1111111110000000");
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

	pub trait Addressable {
		type Address;
		type Error;

		fn len(&self) -> usize;

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

pub mod ins {
	pub mod format {
		macro_rules! instruction_format {
			(
				$size:ty => $format:ident (
					$(
						// Field: <name>[(sign@<sign_bit>)? (shl <amount> =>)? lo:hi (| lo:hi)*]: <type>
						$field:ident [
							// OPT: From where to take the sign extend bit
							$( sign @ $sign:literal => )?
							// OPT: Shift left amount before parsing fields
							$( shl $shl:literal => )?
							$( $lo:literal : $hi:literal $( @ $start:literal )? )|+
							// OPT: Cast steps which will be applied in sequence
							// This is primarily used to sign extend from `i32`
							// to `i64`.
						] : $( $( as $step:ty )+ => )* $fsize:ty
					),+
				)
			) => {
				#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
				pub struct $format {
					$(
						pub $field: $fsize,
					)+
				}

				impl $format {
					pub fn parse(size: $size) -> Self {
						$(
							let mut $field = 0;
							{
								$(
									// "Sign extend" when bit is given
									if size >> $sign == 1 {
										$field = !0;
									}
								)?
								$(
									$field <<= $shl;
								)?
								$(
									let mask = ((1 << (($hi - $lo) + 1)) - 1);
									// Has bits which will be set as `0` and all untoched
									// bits will be `1`.
									let zero_out_mask = !(mask $( << $start )? );

									// Bits to be copied from size to $value.
									let copy_bits = ((size >> $lo) & mask);

									// Bits shifted to the requested location in $value.
									let copy_bits_relocated = copy_bits $( << $start )?;

									// Clear mask bits in $field (in case of 1 sign-extend above)
									// Set bits to requested bits
									$field = ($field & zero_out_mask) |  copy_bits_relocated;
								)+
							}
						)+

						Self {
							$(
								$field: $field $( $( as $step )+ )* as $fsize ,
							)+
						}
					}
				}
			}
		}

		// TODO: maybe replace with handwritten ones for perf?

		instruction_format!(u32 => FormatR(rd[7:11]: u8, rs1[15:19]: u8, rs2[20:24]: u8));
		instruction_format!(u32 => FormatI(rd[7:11]: u8, rs1[15:19]: u8,                 imm[sign@31 => 20:31]: as i32 => i64));
		instruction_format!(u32 => FormatS(              rs1[15:19]: u8, rs2[20:24]: u8, imm[sign@31 => 7:11 @ 0 | 25:31 @ 5]: as i32 => i64));
		instruction_format!(u32 => FormatB(              rs1[15:19]: u8, rs2[20:24]: u8, imm[sign@31 => shl 1 => 8:11 @ 1 | 25:30 @ 5 | 7:7 @ 11 | 31:31 @ 12]: as i32 as i64 => u64));
		instruction_format!(u32 => FormatU(rd[7:11]: u8,                                 imm[sign@31 => shl 12 => 12:31]: as i32 => i64));
		instruction_format!(u32 => FormatJ(rd[7:11]: u8,                                 imm[sign@31 => shl 1 => 21:30 @ 1 | 20:20 @ 11 | 12:19 @ 12 | 31:31 @ 20]: as i32 as i64 => u64));
	}

	use crate::cpu::Cpu;
	use crate::tra::Trap;

	// Currently either 32 or 16 bits
	//
	// # Illegal instructions:
	//
	// - [15:0] all 0
	// - [ILEN-1:0] all 1
	//
	// # Construction
	//
	// Put together from 16bit `parcels` (meaning an instruction must be a multiple of 16).
	//
	// These contents of a `parcel` are stored in little endian regardless of system endianness.
	// An instruction consisting of multiple parcels is stored in little endian.
	//
	// [p1 - lowest][p2 - middle][p3 - highest]
	//
	// # Immediates
	//
	// Immediates are always sign-extended (Exception: 5-bit CSR instructions).
	// The sign bit for immediates is always the 31st bit.
	pub struct Instruction {
		mask: u32,
		reqd: u32,
		name: &'static str,
		extension: &'static str,
		op: fn(cpu: &mut Cpu, word: u32) -> Result<(), Trap>,
	}

	pub const INSTRUCTIONS: [Instruction; 53] = [
		// RV32I
		Instruction {
			//      imm                rd   op
			mask: 0b000000000000000000_0000_1111111,
			reqd: 0b000000000000000000_0000_0110111,
			name: "LUI",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatU
				Ok(())
			},
		},
		Instruction {
			//      imm                rd   op
			mask: 0b000000000000000000_0000_1111111,
			reqd: 0b000000000000000000_0000_0010111,
			name: "AUIPC",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatU
				Ok(())
			},
		},
		Instruction {
			//      imm                rd   op
			mask: 0b000000000000000000_0000_1111111,
			reqd: 0b000000000000000000_0000_1101111,
			name: "JAL",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatJ
				Ok(())
			},
		},
		Instruction {
			//      imm         rs1  fn3 rd   op
			mask: 0b00000000000_0000_111_0000_1111111,
			reqd: 0b00000000000_0000_000_0000_1100111,
			name: "JALR",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatI
				Ok(())
			},
		},
		Instruction {
			//      imm     rs2  rs1  fn3 imm  op
			mask: 0b0000000_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_000_0000_1100011,
			name: "BEQ",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatB
				Ok(())
			},
		},
		Instruction {
			//      imm     rs2  rs1  fn3 imm  op
			mask: 0b0000000_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_001_0000_1100011,
			name: "BNQ",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatB
				Ok(())
			},
		},
		Instruction {
			//      imm     rs2  rs1  fn3 imm  op
			mask: 0b0000000_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_100_0000_1100011,
			name: "BLT",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatB
				Ok(())
			},
		},
		Instruction {
			//      imm     rs2  rs1  fn3 imm  op
			mask: 0b0000000_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_101_0000_1100011,
			name: "BGE",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatB
				Ok(())
			},
		},
		Instruction {
			//      imm     rs2  rs1  fn3 imm  op
			mask: 0b0000000_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_110_0000_1100011,
			name: "BLTU",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatB
				Ok(())
			},
		},
		Instruction {
			//      imm     rs2  rs1  fn3 imm  op
			mask: 0b0000000_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_111_0000_1100011,
			name: "BGEU",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatB
				Ok(())
			},
		},
		Instruction {
			//      imm         rs1  fn3 rd   op
			mask: 0b00000000000_0000_111_0000_1111111,
			reqd: 0b00000000000_0000_000_0000_0000011,
			name: "LB",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatI
				Ok(())
			},
		},
		Instruction {
			//      imm         rs1  fn3 rd   op
			mask: 0b00000000000_0000_111_0000_1111111,
			reqd: 0b00000000000_0000_001_0000_0000011,
			name: "LH",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatI
				Ok(())
			},
		},
		Instruction {
			//      imm         rs1  fn3 rd   op
			mask: 0b00000000000_0000_111_0000_1111111,
			reqd: 0b00000000000_0000_010_0000_0000011,
			name: "LW",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatI
				Ok(())
			},
		},
		Instruction {
			//      imm         rs1  fn3 rd   op
			mask: 0b00000000000_0000_111_0000_1111111,
			reqd: 0b00000000000_0000_100_0000_0000011,
			name: "LBU",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatI
				Ok(())
			},
		},
		Instruction {
			//      imm         rs1  fn3 rd   op
			mask: 0b00000000000_0000_111_0000_1111111,
			reqd: 0b00000000000_0000_101_0000_0000011,
			name: "LHU",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatI
				Ok(())
			},
		},
		Instruction {
			//      imm     rs2  rs1  fn3 imm  op
			mask: 0b0000000_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_000_0000_0100011,
			name: "SB",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatS
				Ok(())
			},
		},
		Instruction {
			//      imm     rs2  rs1  fn3 imm  op
			mask: 0b0000000_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_001_0000_0100011,
			name: "SH",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatS
				Ok(())
			},
		},
		Instruction {
			//      imm     rs2  rs1  fn3 imm  op
			mask: 0b0000000_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_010_0000_0100011,
			name: "SW",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatS
				Ok(())
			},
		},
		Instruction {
			//      imm         rs1  fn3 rd   op
			mask: 0b00000000000_0000_111_0000_1111111,
			reqd: 0b00000000000_0000_000_0000_0010011,
			name: "ADDI",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatI
				Ok(())
			},
		},
		Instruction {
			//      imm         rs1  fn3 rd   op
			mask: 0b00000000000_0000_111_0000_1111111,
			reqd: 0b00000000000_0000_010_0000_0010011,
			name: "SLTI",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatI
				Ok(())
			},
		},
		Instruction {
			//      imm         rs1  fn3 rd   op
			mask: 0b00000000000_0000_111_0000_1111111,
			reqd: 0b00000000000_0000_011_0000_0010011,
			name: "SLTIU",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatI
				Ok(())
			},
		},
		Instruction {
			//      imm         rs1  fn3 rd   op
			mask: 0b00000000000_0000_111_0000_1111111,
			reqd: 0b00000000000_0000_100_0000_0010011,
			name: "XORI",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatI
				Ok(())
			},
		},
		Instruction {
			//      imm         rs1  fn3 rd   op
			mask: 0b00000000000_0000_111_0000_1111111,
			reqd: 0b00000000000_0000_110_0000_0010011,
			name: "ORI",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatI
				Ok(())
			},
		},
		Instruction {
			//      imm         rs1  fn3 rd   op
			mask: 0b00000000000_0000_111_0000_1111111,
			reqd: 0b00000000000_0000_111_0000_0010011,
			name: "ANDI",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatI
				Ok(())
			},
		},
		/**
		 * Disabled in favour of RV64I versions
		Instruction {
			//      imm     shct rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_001_0000_0010011,
			name: "SLLI",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatI (special)
				Ok(())
			},
		},
		Instruction {
			//      imm     shct rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_101_0000_0010011,
			name: "SRLI",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatI (special)
				Ok(())
			},
		},
		Instruction {
			//      imm     shct rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0100000_0000_0000_101_0000_0010011,
			name: "SRAI",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatI (special)
				Ok(())
			},
		},
		*/
		Instruction {
			//      fn7     rs2  rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_000_0000_0110011,
			name: "ADD",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},
		Instruction {
			//      fn7     rs2  rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0100000_0000_0000_000_0000_0110011,
			name: "SUB",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},
		Instruction {
			//      fn7     rs2  rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_001_0000_0110011,
			name: "SLL",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},
		Instruction {
			//      fn7     rs2  rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_010_0000_0110011,
			name: "SLT",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},
		Instruction {
			//      fn7     rs2  rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_011_0000_0110011,
			name: "SLTU",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},
		Instruction {
			//      fn7     rs2  rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_100_0000_0110011,
			name: "XOR",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},
		Instruction {
			//      fn7     rs2  rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_101_0000_0110011,
			name: "SRL",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},
		Instruction {
			//      fn7     rs2  rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0100000_0000_0000_101_0000_0110011,
			name: "SRA",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},
		Instruction {
			//      fn7     rs2  rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_110_0000_0110011,
			name: "OR",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},
		Instruction {
			//      fn7     rs2  rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_111_0000_0110011,
			name: "AND",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},
		Instruction {
			//      fm  pred suc  rs1  fn3 rd   op
			mask: 0b000_0000_0000_0000_111_0000_1111111,
			reqd: 0b000_0000_0000_0000_000_0000_0001111,
			name: "FENCE",
			extension: "RV32I",
			op: |cpu, word| {
				// Special
				Ok(())
			},
		},
		Instruction {
			//      imm         rs1  fn3 rd   op
			mask: 0b11111111111_1111_111_1111_1111111,
			reqd: 0b00000000000_0000_000_0000_1110011,
			name: "ECALL",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatI
				Ok(())
			},
		},
		Instruction {
			//      imm         rs1  fn3 rd   op
			mask: 0b11111111111_1111_111_1111_1111111,
			reqd: 0b00000000001_0000_000_0000_1110011,
			name: "EBREAK",
			extension: "RV32I",
			op: |cpu, word| {
				// FormatI
				Ok(())
			},
		},
		// RV64I
		Instruction {
			//      imm         rs1  fn3 rd   op
			mask: 0b00000000000_0000_111_0000_1111111,
			reqd: 0b00000000000_0000_110_0000_0000011,
			name: "LWU",
			extension: "RV64I",
			op: |cpu, word| {
				// FormatI
				Ok(())
			},
		},
		Instruction {
			//      imm         rs1  fn3 rd   op
			mask: 0b00000000000_0000_111_0000_1111111,
			reqd: 0b00000000000_0000_011_0000_0000011,
			name: "LD",
			extension: "RV64I",
			op: |cpu, word| {
				// FormatI
				Ok(())
			},
		},
		Instruction {
			//      imm     rs2  rs1  fn3 imm  op
			mask: 0b0000000_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_011_0000_0100011,
			name: "SD",
			extension: "RV64I",
			op: |cpu, word| {
				// FormatS
				Ok(())
			},
		},
		Instruction {
			//      fn7    shamt rs1  fn3 rd   op
			mask: 0b111111_00000_0000_111_0000_1111111,
			reqd: 0b000000_00000_0000_001_0000_0010011,
			name: "SLLI",
			extension: "RV64I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},
		Instruction {
			//      fn7    shamt rs1  fn3 rd   op
			mask: 0b111111_00000_0000_111_0000_1111111,
			reqd: 0b000000_00000_0000_101_0000_0010011,
			name: "SRLI",
			extension: "RV64I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},
		Instruction {
			//      fn7    shamt rs1  fn3 rd   op
			mask: 0b111111_00000_0000_111_0000_1111111,
			reqd: 0b010000_00000_0000_101_0000_0010011,
			name: "SRAI",
			extension: "RV64I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},
		Instruction {
			//      imm         rs1  fn3 rd   op
			mask: 0b00000000000_0000_111_0000_1111111,
			reqd: 0b00000000000_0000_000_0000_0011011,
			name: "ADDIW",
			extension: "RV64I",
			op: |cpu, word| {
				// FormatI
				Ok(())
			},
		},
		Instruction {
			//      fn7     shamt rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_001_0000_0011011,
			name: "SLLIW",
			extension: "RV64I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},
		Instruction {
			//      fn7     shamt rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_101_0000_0011011,
			name: "SRLIW",
			extension: "RV64I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},
		Instruction {
			//      fn7     shamt rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0100000_0000_0000_101_0000_0011011,
			name: "SRAIW",
			extension: "RV64I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},
		Instruction {
			//      fn7     shamt rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_000_0000_0111011,
			name: "ADDW",
			extension: "RV64I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},
		Instruction {
			//      fn7     shamt rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0100000_0000_0000_000_0000_0111011,
			name: "SUBW",
			extension: "RV64I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},
		Instruction {
			//      fn7     shamt rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_001_0000_0111011,
			name: "SLLW",
			extension: "RV64I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},
		Instruction {
			//      fn7     shamt rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0000000_0000_0000_101_0000_0111011,
			name: "SRLW",
			extension: "RV64I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},
		Instruction {
			//      fn7     shamt rs1  fn3 rd   op
			mask: 0b1111111_0000_0000_111_0000_1111111,
			reqd: 0b0100000_0000_0000_101_0000_0111011,
			name: "SRAW",
			extension: "RV64I",
			op: |cpu, word| {
				// FormatR
				Ok(())
			},
		},

		// RV32/RV64 Zifencei
		Instruction {
			//      imm         rs1  fn3 rd   op
			mask: 0b00000000000_0000_111_0000_1111111,
			reqd: 0b00000000000_0000_001_0000_0001111,
			name: "FENCE.I",
			extension: "Zifencei",
			op: |cpu, word| {
				// FormatI
				Ok(())
			},
		},
	];

	#[test]
	fn decode() {
		let word = 0b0000000_1010_1010_000_1010_0110011;

		for instr in &INSTRUCTIONS {
			if word & instr.mask == instr.reqd {
				println!("{}", instr.name);
				break;
			}
		}
	}

	#[test]
	fn unique_instruction_names() {
		use std::collections::HashMap;

		let mut names: HashMap<&'static str, usize> = HashMap::new();

		for instr in &INSTRUCTIONS {
			*names.entry(instr.name).or_default() += 1;
		}

		let mut duplicates = false;
		for (name, count) in names.into_iter().filter(|(_, v)| v > &1) {
			println!("Duplicate for name `{name}`: {count}");
			duplicates = true;
		}

		assert!(!duplicates, "Found duplicate names");
	}

	#[test]
	fn valid_masks() {
		for instr in &INSTRUCTIONS {
			assert_eq!(
				instr.reqd & instr.mask,
				instr.reqd,
				"Invalid mask and required bits for instruction {}",
				instr.name
			);
			assert_eq!(
				instr.reqd | instr.mask,
				instr.mask,
				"Invalid mask and required bits for instruction {}",
				instr.name
			);
		}
	}

	#[test]
	#[ignore = "Takes very very long to run and maxes out the whole cpu. Only \
	            run when the instructions change."]
	fn unique_instruction_codes() {
		use rayon::prelude::*;

		(0..=u32::MAX).into_par_iter().for_each(|word| {
			let mut found: Option<&Instruction> = None;

			for instr in &INSTRUCTIONS {
				if word & instr.mask == instr.reqd {
					if let Some(f) = found {
						panic!(
							"Found duplicate instruction code for `{}` and \
							 `{}`",
							instr.name, f.name
						);
					} else {
						found = Some(instr);
					}
				}
			}
		})
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

	#[allow(non_camel_case_types)]
	#[repr(usize)]
	#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
	pub enum IntReg {
		// Always `0`.
		x0,
		// Used to hold return address.
		x1,
		// Used stack pointer.
		x2,
		x3,
		x4,
		// Alternative link register for x1.
		x5,
		x6,
		x7,
		x8,
		x9,
		x10,
		x11,
		x12,
		x13,
		x14,
		x15,
		x16,
		x17,
		x18,
		x19,
		x20,
		x21,
		x22,
		x23,
		x24,
		x25,
		x26,
		x27,
		x28,
		x29,
		x30,
		x31,
	}

	impl From<IntReg> for usize {
		fn from(value: IntReg) -> Self {
			value as usize
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

	#[allow(non_camel_case_types)]
	#[repr(usize)]
	#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
	pub enum FloatReg {
		f0,
		f1,
		f2,
		f3,
		f4,
		f5,
		f6,
		f7,
		f8,
		f9,
		f10,
		f11,
		f12,
		f13,
		f14,
		f15,
		f16,
		f17,
		f18,
		f19,
		f20,
		f21,
		f22,
		f23,
		f24,
		f25,
		f26,
		f27,
		f28,
		f29,
		f30,
		f31,
	}

	impl From<FloatReg> for usize {
		fn from(value: FloatReg) -> Self {
			value as usize
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
	use crate::shared::{IntWidth, XLEN};

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
