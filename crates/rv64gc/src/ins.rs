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
								if (size & (1 << $sign)) > 0 {
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

	instruction_format!(u32 => FormatR( rd[7:11]: u8, rs1[15:19]: u8, rs2[20:24]: u8));
	instruction_format!(u32 => FormatI( rd[7:11]: u8, rs1[15:19]: u8,                 imm[sign@31 => 20:31]: as i32 => i64));
	instruction_format!(u32 => FormatS(               rs1[15:19]: u8, rs2[20:24]: u8, imm[sign@31 => 7:11 @ 0 | 25:31 @ 5]: as i32 => i64));
	instruction_format!(u32 => FormatB(               rs1[15:19]: u8, rs2[20:24]: u8, imm[sign@31 => shl 1 => 8:11 @ 1 | 25:30 @ 5 | 7:7 @ 11 | 31:31 @ 12]: as i32 as i64 => u64));
	instruction_format!(u32 => FormatU( rd[7:11]: u8,                                 imm[sign@31 => shl 12 => 12:31]: as i32 as i64 => u64));
	instruction_format!(u32 => FormatJ( rd[7:11]: u8,                                 imm[sign@31 => shl 1 => 21:30 @ 1 | 20:20 @ 11 | 12:19 @ 12 | 31:31 @ 20]: as i32 as i64 => u64));

	instruction_format!(u32 => FormatR4(rd[7:11]: u8, rs1[15:19]: u8, rs2[20:24]: u8, rs3[27:31]: u8));
}

use self::format::{FormatB, FormatI, FormatJ, FormatR, FormatS};
use crate::adr::Addressable;
use crate::cpu::Cpu;
use crate::ins::format::FormatU;
use crate::reg::IntReg;
use crate::shared::Address;
use crate::tra::Trap;

fn resolve_xreg(cpu: &mut Cpu, reg: u8) -> IntReg {
	IntReg::try_from(reg).unwrap()
}

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
	pub(crate) mask: u32,
	pub(crate) reqd: u32,
	pub(crate) name: &'static str,
	pub(crate) extension: &'static str,
	pub(crate) op:
		fn(cpu: &mut Cpu, word: u32, address: Address) -> Result<(), Trap>,
}

#[allow(
	unused_doc_comments,
	clippy::unusual_byte_groupings,
	clippy::tabs_in_doc_comments
)]
pub const INSTRUCTIONS: [Instruction; 158] = [
	// RV32I
	Instruction {
		//      imm                  rd    op
		mask: 0b00000000000000000000_00000_1111111,
		reqd: 0b00000000000000000000_00000_0110111,
		// Load upper immediate
		name: "LUI",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatU { rd, imm } = FormatU::parse(word);
			let rd = resolve_xreg(cpu, rd);

			cpu.xregs[rd] = imm as i64;

			Ok(())
		},
	},
	Instruction {
		//      imm                  rd    op
		mask: 0b00000000000000000000_00000_1111111,
		reqd: 0b00000000000000000000_00000_0010111,
		// Add upper immediate to pc
		name: "AUIPC",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatU { rd, imm } = FormatU::parse(word);
			let rd = resolve_xreg(cpu, rd);

			cpu.xregs[rd] = cpu.pc.wrapping_add(imm) as i64;

			Ok(())
		},
	},
	Instruction {
		//      imm                  rd    op
		mask: 0b00000000000000000000_00000_1111111,
		reqd: 0b00000000000000000000_00000_1101111,
		// Jump and link
		name: "JAL",
		extension: "RV32I",
		op: |cpu, word, addr| {
			let FormatJ { rd, imm } = FormatJ::parse(word);
			let rd = resolve_xreg(cpu, rd);

			// TODO: add return-address prediciton? See spec page 21/39 bottom.

			// TODO: Check that pc advanced (should be instr + 4).
			cpu.xregs[rd] = cpu.pc as i64;
			cpu.pc = addr.wrapping_add(imm);

			Ok(())
		},
	},
	Instruction {
		//      imm          rs1   fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_000_00000_1100111,
		// Jump and link register
		name: "JALR",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatI { rd, rs1, imm } = FormatI::parse(word);
			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);

			let addr = (imm as u64).wrapping_add(cpu.xregs[rs1] as u64)
				// Set least-significant bit to `0`
				& !1;

			// Should throw exception (instruction-address-misaligned) but not
			// when also processing the `C` extension. That's why the check is
			// skipped here.

			// TODO: Check that pc advanced (should be instr + 4).
			cpu.xregs[rd] = cpu.pc as i64;
			cpu.pc = addr;

			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   fn3 imm   op
		mask: 0b0000000_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_000_00000_1100011,
		// Branch equal
		name: "BEQ",
		extension: "RV32I",
		op: |cpu, word, addr| {
			let FormatB { rs1, rs2, imm } = FormatB::parse(word);
			let rs1 = resolve_xreg(cpu, rs1);
			let rs2 = resolve_xreg(cpu, rs2);

			if cpu.xregs[rs1] == cpu.xregs[rs2] {
				cpu.pc = addr.wrapping_add(imm);
			}

			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   fn3 imm   op
		mask: 0b0000000_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_001_00000_1100011,
		// Branch not equal
		name: "BNQ",
		extension: "RV32I",
		op: |cpu, word, addr| {
			let FormatB { rs1, rs2, imm } = FormatB::parse(word);
			let rs1 = resolve_xreg(cpu, rs1);
			let rs2 = resolve_xreg(cpu, rs2);

			if cpu.xregs[rs1] != cpu.xregs[rs2] {
				cpu.pc = addr.wrapping_add(imm);
			}

			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   fn3 imm   op
		mask: 0b0000000_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_100_00000_1100011,
		// Branch less than
		name: "BLT",
		extension: "RV32I",
		op: |cpu, word, addr| {
			let FormatB { rs1, rs2, imm } = FormatB::parse(word);
			let rs1 = resolve_xreg(cpu, rs1);
			let rs2 = resolve_xreg(cpu, rs2);

			if cpu.xregs[rs1] < cpu.xregs[rs2] {
				cpu.pc = addr.wrapping_add(imm);
			}

			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   fn3 imm   op
		mask: 0b0000000_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_101_00000_1100011,
		// Branch greater than
		name: "BGE",
		extension: "RV32I",
		op: |cpu, word, addr| {
			let FormatB { rs1, rs2, imm } = FormatB::parse(word);
			let rs1 = resolve_xreg(cpu, rs1);
			let rs2 = resolve_xreg(cpu, rs2);

			if (cpu.xregs[rs1] as u64) > (cpu.xregs[rs2] as u64) {
				cpu.pc = addr.wrapping_add(imm);
			}

			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   fn3 imm   op
		mask: 0b0000000_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_110_00000_1100011,
		// Branch less than unsigned
		name: "BLTU",
		extension: "RV32I",
		op: |cpu, word, addr| {
			let FormatB { rs1, rs2, imm } = FormatB::parse(word);
			let rs1 = resolve_xreg(cpu, rs1);
			let rs2 = resolve_xreg(cpu, rs2);

			if (cpu.xregs[rs1] as u64) < (cpu.xregs[rs2] as u64) {
				cpu.pc = addr.wrapping_add(imm);
			}

			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   fn3 imm   op
		mask: 0b0000000_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_111_00000_1100011,
		// Branch greater than unsigned
		name: "BGEU",
		extension: "RV32I",
		op: |cpu, word, addr| {
			let FormatB { rs1, rs2, imm } = FormatB::parse(word);
			let rs1 = resolve_xreg(cpu, rs1);
			let rs2 = resolve_xreg(cpu, rs2);

			if (cpu.xregs[rs1] as u64) > (cpu.xregs[rs2] as u64) {
				cpu.pc = addr.wrapping_add(imm);
			}

			Ok(())
		},
	},
	Instruction {
		//      imm         rs1   fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_000_00000_0000011,
		// Load byte
		name: "LB",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatI { rd, rs1, imm } = FormatI::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);

			let rs1_value = cpu.xregs[rs1];

			let addr = (rs1_value as u64).wrapping_add(imm as u64);

			cpu.xregs[rd] = cpu.mmu.read_u8(addr)? as i8 as i64;

			Ok(())
		},
	},
	Instruction {
		//      imm         rs1   fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_001_00000_0000011,
		// Load half-word
		name: "LH",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatI { rd, rs1, imm } = FormatI::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);

			let rs1_value = cpu.xregs[rs1];

			let addr = (rs1_value as u64).wrapping_add(imm as u64);

			cpu.xregs[rd] = cpu.mmu.read_u16_le(addr)? as i16 as i64;

			Ok(())
		},
	},
	Instruction {
		//      imm          rs1   fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_010_00000_0000011,
		// Load word
		name: "LW",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatI { rd, rs1, imm } = FormatI::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);

			let rs1_value = cpu.xregs[rs1];

			let addr = (rs1_value as u64).wrapping_add(imm as u64);

			cpu.xregs[rd] = cpu.mmu.read_u32_le(addr)? as i32 as i64;

			Ok(())
		},
	},
	Instruction {
		//      imm          rs1   fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_100_00000_0000011,
		// Load byte unsigned
		name: "LBU",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatI { rd, rs1, imm } = FormatI::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);

			let rs1_value = cpu.xregs[rs1];

			let addr = (rs1_value as u64).wrapping_add(imm as u64);

			cpu.xregs[rd] = cpu.mmu.read_u8(addr)? as i64;

			Ok(())
		},
	},
	Instruction {
		//      imm          rs1   fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_101_00000_0000011,
		// Load half-word unsigned
		name: "LHU",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatI { rd, rs1, imm } = FormatI::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);

			let rs1_value = cpu.xregs[rs1];

			let addr = (rs1_value as u64).wrapping_add(imm as u64);

			cpu.xregs[rd] = cpu.mmu.read_u16_le(addr)? as i64;

			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   fn3 imm   op
		mask: 0b0000000_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_000_00000_0100011,
		// Store byte
		name: "SB",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatS { rs1, rs2, imm } = FormatS::parse(word);

			let rs1 = resolve_xreg(cpu, rs1);
			let rs2 = resolve_xreg(cpu, rs2);

			let rs1_value = cpu.xregs[rs1];

			let addr = (rs1_value as u64).wrapping_add(imm as u64);

			cpu.mmu.write_u8(addr, cpu.xregs[rs2] as u8)?;

			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   fn3 imm   op
		mask: 0b0000000_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_001_00000_0100011,
		// Store half-word
		name: "SH",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatS { rs1, rs2, imm } = FormatS::parse(word);

			let rs1 = resolve_xreg(cpu, rs1);
			let rs2 = resolve_xreg(cpu, rs2);

			let rs1_value = cpu.xregs[rs1];

			let addr = (rs1_value as u64).wrapping_add(imm as u64);

			cpu.mmu.write_u16_le(addr, cpu.xregs[rs2] as u16)?;

			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   fn3 imm   op
		mask: 0b0000000_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_010_00000_0100011,
		// Store word
		name: "SW",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatS { rs1, rs2, imm } = FormatS::parse(word);

			let rs1 = resolve_xreg(cpu, rs1);
			let rs2 = resolve_xreg(cpu, rs2);

			let rs1_value = cpu.xregs[rs1];

			let addr = (rs1_value as u64).wrapping_add(imm as u64);

			cpu.mmu.write_u32_le(addr, cpu.xregs[rs2] as u32)?;

			Ok(())
		},
	},
	Instruction {
		//      imm          rs1   fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_000_00000_0010011,
		// Add immediate
		name: "ADDI",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatI { rd, rs1, imm } = FormatI::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);

			let rs1_value = cpu.xregs[rs1];

			cpu.xregs[rd] = rs1_value.wrapping_add(imm);

			Ok(())
		},
	},
	Instruction {
		//      imm          rs1   fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_010_00000_0010011,
		// Set less than immediate
		name: "SLTI",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatI { rd, rs1, imm } = FormatI::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);

			let rs1_value = cpu.xregs[rs1];

			if rs1_value < imm {
				cpu.xregs[rd] = 1;
			} else {
				cpu.xregs[rd] = 0;
			}

			Ok(())
		},
	},
	Instruction {
		//      imm          rs1   fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_011_00000_0010011,
		// Set less than immediate unsigned
		name: "SLTIU",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatI { rd, rs1, imm } = FormatI::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);

			let rs1_value = cpu.xregs[rs1];

			if (rs1_value as u64) < (imm as u64) {
				cpu.xregs[rd] = 1;
			} else {
				cpu.xregs[rd] = 0;
			}

			Ok(())
		},
	},
	Instruction {
		//      imm          rs1   fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_100_00000_0010011,
		// Xor immediate
		name: "XORI",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatI { rd, rs1, imm } = FormatI::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);

			let rs1_value = cpu.xregs[rs1];

			cpu.xregs[rd] = rs1_value ^ imm;

			Ok(())
		},
	},
	Instruction {
		//      imm          rs1   fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_110_00000_0010011,
		// Or immediate
		name: "ORI",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatI { rd, rs1, imm } = FormatI::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);

			let rs1_value = cpu.xregs[rs1];

			cpu.xregs[rd] = rs1_value | imm;

			Ok(())
		},
	},
	Instruction {
		//      imm          rs1   fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_111_00000_0010011,
		// And immediate
		name: "ANDI",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatI { rd, rs1, imm } = FormatI::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);

			let rs1_value = cpu.xregs[rs1];

			cpu.xregs[rd] = rs1_value & imm;

			Ok(())
		},
	},
	/**
	 * Disabled in favour of RV64I versions
	Instruction {
		//      imm     shct  rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_001_00000_0010011,
		name: "SLLI",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			// FormatI (special)
			Ok(())
		},
	},
	Instruction {
		//      imm     shct  rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_101_00000_0010011,
		name: "SRLI",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			// FormatI (special)
			Ok(())
		},
	},
	Instruction {
		//      imm     shct  rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0100000_00000_00000_101_00000_0010011,
		name: "SRAI",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			// FormatI (special)
			Ok(())
		},
	},
	*/
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_000_00000_0110011,
		name: "ADD",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatR { rd, rs1, rs2 } = FormatR::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);
			let rs2 = resolve_xreg(cpu, rs2);

			let rs1_value = cpu.xregs[rs1];
			let rs2_value = cpu.xregs[rs2];

			cpu.xregs[rd] = rs1_value.wrapping_add(rs2_value);

			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0100000_00000_00000_000_00000_0110011,
		name: "SUB",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatR { rd, rs1, rs2 } = FormatR::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);
			let rs2 = resolve_xreg(cpu, rs2);

			let rs1_value = cpu.xregs[rs1];
			let rs2_value = cpu.xregs[rs2];

			cpu.xregs[rd] = rs1_value.wrapping_sub(rs2_value);

			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_001_00000_0110011,
		// Shift left logical
		name: "SLL",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatR { rd, rs1, rs2 } = FormatR::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);
			let rs2 = resolve_xreg(cpu, rs2);

			let rs1_value = cpu.xregs[rs1];
			let rs2_value = cpu.xregs[rs2];

			// `rs2` is 5-bit long and thus can not exceed u32.
			cpu.xregs[rd] =
				(rs1_value as u64).wrapping_shl(rs2_value as u32) as i64;

			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_010_00000_0110011,
		// Signed less than
		name: "SLT",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatR { rd, rs1, rs2 } = FormatR::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);
			let rs2 = resolve_xreg(cpu, rs2);

			let rs1_value = cpu.xregs[rs1];
			let rs2_value = cpu.xregs[rs2];

			if rs1_value < rs2_value {
				cpu.xregs[rd] = 1;
			} else {
				cpu.xregs[rd] = 0;
			}

			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_011_00000_0110011,
		// Signed less than (unsigned)
		name: "SLTU",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatR { rd, rs1, rs2 } = FormatR::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);
			let rs2 = resolve_xreg(cpu, rs2);

			let rs1_value = cpu.xregs[rs1];
			let rs2_value = cpu.xregs[rs2];

			if (rs1_value as u64) < (rs2_value as u64) {
				cpu.xregs[rd] = 1;
			} else {
				cpu.xregs[rd] = 0;
			}

			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_100_00000_0110011,
		// Xor
		name: "XOR",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatR { rd, rs1, rs2 } = FormatR::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);
			let rs2 = resolve_xreg(cpu, rs2);

			let rs1_value = cpu.xregs[rs1];
			let rs2_value = cpu.xregs[rs2];

			cpu.xregs[rd] = rs1_value ^ rs2_value;

			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_101_00000_0110011,
		// Shift right logical
		name: "SRL",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatR { rd, rs1, rs2 } = FormatR::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);
			let rs2 = resolve_xreg(cpu, rs2);

			let rs1_value = cpu.xregs[rs1];
			let rs2_value = cpu.xregs[rs2];

			// `rs2` is 5-bit long and thus can not exceed u32.
			cpu.xregs[rd] =
				(rs1_value as u64).wrapping_shr(rs2_value as u32) as i64;

			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0100000_00000_00000_101_00000_0110011,
		// Shift right arithmetic (fill with sign bit instead of `0`)
		name: "SRA",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatR { rd, rs1, rs2 } = FormatR::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);
			let rs2 = resolve_xreg(cpu, rs2);

			let rs1_value = cpu.xregs[rs1];
			let rs2_value = cpu.xregs[rs2];

			// `rs2` is 5-bit long and thus can not exceed u32.
			// TODO: check arithmetic shift
			cpu.xregs[rd] = rs1_value.wrapping_shr(rs2_value as u32);

			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_110_00000_0110011,
		// Or
		name: "OR",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatR { rd, rs1, rs2 } = FormatR::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);
			let rs2 = resolve_xreg(cpu, rs2);

			let rs1_value = cpu.xregs[rs1];
			let rs2_value = cpu.xregs[rs2];

			cpu.xregs[rd] = rs1_value | rs2_value;

			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_111_00000_0110011,
		// And
		name: "AND",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatR { rd, rs1, rs2 } = FormatR::parse(word);

			let rd = resolve_xreg(cpu, rd);
			let rs1 = resolve_xreg(cpu, rs1);
			let rs2 = resolve_xreg(cpu, rs2);

			let rs1_value = cpu.xregs[rs1];
			let rs2_value = cpu.xregs[rs2];

			cpu.xregs[rd] = rs1_value & rs2_value;

			Ok(())
		},
	},
	Instruction {
		//      fm   pred suc  rs1   fn3 rd    op
		mask: 0b0000_0000_0000_00000_111_00000_1111111,
		reqd: 0b0000_0000_0000_00000_000_00000_0001111,
		name: "FENCE",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			// TODO: Impl (with one hart not needed)
			Ok(())
		},
	},
	Instruction {
		//      imm          rs1   fn3 rd    op
		mask: 0b111111111111_11111_111_11111_1111111,
		reqd: 0b000000000000_00000_000_00000_1110011,
		name: "ECALL",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			// FormatI
			let FormatI { rd, rs1, imm } = FormatI::parse(word);

			// TODO: return trap depending on eei

			Ok(())
		},
	},
	Instruction {
		//      imm          rs1   fn3 rd    op
		mask: 0b111111111111_11111_111_11111_1111111,
		reqd: 0b000000000001_00000_000_00000_1110011,
		name: "EBREAK",
		extension: "RV32I",
		op: |cpu, word, _addr| {
			let FormatI { rd, rs1, imm } = FormatI::parse(word);

			// TODO: return trap depending on eei

			Ok(())
		},
	},
	// RV64I
	Instruction {
		//      imm          rs1   fn3 rd    op
		mask: 0b111111111111_11111_111_11111_1111111,
		reqd: 0b000000000000_00000_110_00000_0000011,
		name: "LWU",
		extension: "RV64I",
		op: |cpu, word, _addr| {
			// FormatI
			Ok(())
		},
	},
	Instruction {
		//      imm          rs1   fn3 rd    op
		mask: 0b111111111111_11111_111_11111_1111111,
		reqd: 0b000000000000_00000_011_00000_0000011,
		name: "LD",
		extension: "RV64I",
		op: |cpu, word, _addr| {
			// FormatI
			Ok(())
		},
	},
	Instruction {
		//      imm      rs2   rs1   fn3 imm   op
		mask: 0b00000000_00000_00000_111_00000_1111111,
		reqd: 0b00000000_00000_00000_011_00000_0100011,
		name: "SD",
		extension: "RV64I",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      fn7    shamt  rs1   fn3 rd    op
		mask: 0b111111_000000_00000_111_00000_1111111,
		reqd: 0b000000_000000_00000_001_00000_0010011,
		name: "SLLI",
		extension: "RV64I",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7    shamt  rs1   fn3 rd    op
		mask: 0b111111_000000_00000_111_00000_1111111,
		reqd: 0b000000_000000_00000_101_00000_0010011,
		name: "SRLI",
		extension: "RV64I",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7    shamt  rs1   fn3 rd    op
		mask: 0b111111_000000_00000_111_00000_1111111,
		reqd: 0b010000_000000_00000_101_00000_0010011,
		name: "SRAI",
		extension: "RV64I",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      imm          rs1   fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_000_00000_0011011,
		name: "ADDIW",
		extension: "RV64I",
		op: |cpu, word, _addr| {
			// FormatI
			Ok(())
		},
	},
	Instruction {
		//      fn7     shamt rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_001_00000_0011011,
		name: "SLLIW",
		extension: "RV64I",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7     shamt rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_101_00000_0011011,
		name: "SRLIW",
		extension: "RV64I",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7     shamt rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0100000_00000_00000_101_00000_0011011,
		name: "SRAIW",
		extension: "RV64I",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7     shamt rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_000_00000_0111011,
		name: "ADDW",
		extension: "RV64I",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7     shamt rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0100000_00000_00000_000_00000_0111011,
		name: "SUBW",
		extension: "RV64I",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7     shamt rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_001_00000_0111011,
		name: "SLLW",
		extension: "RV64I",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7     shamt rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_101_00000_0111011,
		name: "SRLW",
		extension: "RV64I",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7     shamt rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0100000_00000_00000_101_00000_0111011,
		name: "SRAW",
		extension: "RV64I",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	// RV32/RV64 Zifencei
	Instruction {
		//      imm          rs1   fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_001_00000_0001111,
		name: "FENCE.I",
		extension: "Zifencei",
		op: |cpu, word, _addr| {
			// FormatI
			Ok(())
		},
	},
	// RV32/RV64 Zicsr
	Instruction {
		//      csr          rs1   fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_001_00000_1110011,
		name: "CSRRW",
		extension: "Zicsr",
		op: |cpu, word, _addr| {
			// FormatI
			Ok(())
		},
	},
	Instruction {
		//      csr          rs1   fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_010_00000_1110011,
		name: "CSRRS",
		extension: "Zicsr",
		op: |cpu, word, _addr| {
			// FormatI
			Ok(())
		},
	},
	Instruction {
		//      csr          rs1   fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_011_00000_1110011,
		name: "CSRRC",
		extension: "Zicsr",
		op: |cpu, word, _addr| {
			// FormatI
			Ok(())
		},
	},
	Instruction {
		//      csr          uimm  fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_101_00000_1110011,
		name: "CSRRWI",
		extension: "Zicsr",
		op: |cpu, word, _addr| {
			// FormatI
			Ok(())
		},
	},
	Instruction {
		//      csr          uimm  fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_110_00000_1110011,
		name: "CSRRSI",
		extension: "Zicsr",
		op: |cpu, word, _addr| {
			// FormatI
			Ok(())
		},
	},
	Instruction {
		//      csr          uimm  fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_111_00000_1110011,
		name: "CSRRCI",
		extension: "Zicsr",
		op: |cpu, word, _addr| {
			// FormatI
			Ok(())
		},
	},
	// RV32M
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000001_00000_00000_000_00000_0110011,
		name: "MUL",
		extension: "RV32M",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000001_00000_00000_001_00000_0110011,
		name: "MULH",
		extension: "RV32M",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000001_00000_00000_010_00000_0110011,
		name: "MULHSU",
		extension: "RV32M",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000001_00000_00000_011_00000_0110011,
		name: "MULHU",
		extension: "RV32M",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000001_00000_00000_100_00000_0110011,
		name: "DIV",
		extension: "RV32M",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000001_00000_00000_101_00000_0110011,
		name: "DIVU",
		extension: "RV32M",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000001_00000_00000_110_00000_0110011,
		name: "REM",
		extension: "RV32M",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000001_00000_00000_111_00000_0110011,
		name: "REMU",
		extension: "RV32M",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	// RV64M
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000001_00000_00000_000_00000_0111011,
		name: "MULW",
		extension: "RV64M",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000001_00000_00000_100_00000_0111011,
		name: "DIVW",
		extension: "RV64M",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000001_00000_00000_101_00000_0111011,
		name: "DIVUW",
		extension: "RV64M",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000001_00000_00000_110_00000_0111011,
		name: "REMW",
		extension: "RV64M",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0000001_00000_00000_111_00000_0111011,
		name: "REMUW",
		extension: "RV64M",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	// RV32A
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_11111_00000_111_00000_1111111,
		reqd: 0b00010_0_0_00000_00000_010_00000_0101111,
		name: "LR.W",
		extension: "RV32A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_00000_00000_111_00000_1111111,
		reqd: 0b00011_0_0_00000_00000_010_00000_0101111,
		name: "SC.W",
		extension: "RV32A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_00000_00000_111_00000_1111111,
		reqd: 0b00001_0_0_00000_00000_010_00000_0101111,
		name: "AMOSWAP.W",
		extension: "RV32A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_00000_00000_111_00000_1111111,
		reqd: 0b00000_0_0_00000_00000_010_00000_0101111,
		name: "AMOADD.W",
		extension: "RV32A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_00000_00000_111_00000_1111111,
		reqd: 0b00100_0_0_00000_00000_010_00000_0101111,
		name: "AMOXOR.W",
		extension: "RV32A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_00000_00000_111_00000_1111111,
		reqd: 0b01100_0_0_00000_00000_010_00000_0101111,
		name: "AMOAND.W",
		extension: "RV32A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_00000_00000_111_00000_1111111,
		reqd: 0b01000_0_0_00000_00000_010_00000_0101111,
		name: "AMOOR.W",
		extension: "RV32A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_00000_00000_111_00000_1111111,
		reqd: 0b10000_0_0_00000_00000_010_00000_0101111,
		name: "AMOMIN.W",
		extension: "RV32A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_00000_00000_111_00000_1111111,
		reqd: 0b10100_0_0_00000_00000_010_00000_0101111,
		name: "AMOMAX.W",
		extension: "RV32A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_00000_00000_111_00000_1111111,
		reqd: 0b11000_0_0_00000_00000_010_00000_0101111,
		name: "AMOMINU.W",
		extension: "RV32A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_00000_00000_111_00000_1111111,
		reqd: 0b11100_0_0_00000_00000_010_00000_0101111,
		name: "AMOMAXU.W",
		extension: "RV32A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	// RV64A
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_11111_00000_111_00000_1111111,
		reqd: 0b00010_0_0_00000_00000_011_00000_0101111,
		name: "LR.D",
		extension: "RV64A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_00000_00000_111_00000_1111111,
		reqd: 0b00011_0_0_00000_00000_011_00000_0101111,
		name: "SC.D",
		extension: "RV64A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_00000_00000_111_00000_1111111,
		reqd: 0b00001_0_0_00000_00000_011_00000_0101111,
		name: "AMOSWAP.D",
		extension: "RV64A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_00000_00000_111_00000_1111111,
		reqd: 0b00000_0_0_00000_00000_011_00000_0101111,
		name: "AMOADD.D",
		extension: "RV64A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_00000_00000_111_00000_1111111,
		reqd: 0b00100_0_0_00000_00000_011_00000_0101111,
		name: "AMOXOR.D",
		extension: "RV64A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_00000_00000_111_00000_1111111,
		reqd: 0b01100_0_0_00000_00000_011_00000_0101111,
		name: "AMOAND.D",
		extension: "RV64A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_00000_00000_111_00000_1111111,
		reqd: 0b01000_0_0_00000_00000_011_00000_0101111,
		name: "AMOOR.D",
		extension: "RV64A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_00000_00000_111_00000_1111111,
		reqd: 0b10000_0_0_00000_00000_011_00000_0101111,
		name: "AMOMIN.D",
		extension: "RV64A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_00000_00000_111_00000_1111111,
		reqd: 0b10100_0_0_00000_00000_011_00000_0101111,
		name: "AMOMAX.D",
		extension: "RV64A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_00000_00000_111_00000_1111111,
		reqd: 0b11000_0_0_00000_00000_011_00000_0101111,
		name: "AMOMINU.D",
		extension: "RV64A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7       rs2   rs1   fn3 rd    op
		mask: 0b11111_0_0_00000_00000_111_00000_1111111,
		reqd: 0b11100_0_0_00000_00000_011_00000_0101111,
		name: "AMOMAXU.D",
		extension: "RV64A",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	// RV32F
	Instruction {
		//      imm          rs1   fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_010_00000_0000111,
		name: "FLW",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatI
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   fn3 imm    op
		mask: 0b0000000_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_010_00000_0100111,
		name: "FSW",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      rs3      rs2   rs1   rm  rd    op
		mask: 0b00000_11_00000_00000_000_00000_1111111,
		reqd: 0b00000_00_00000_00000_000_00000_1000011,
		name: "FMADD.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      rs3      rs2   rs1   rm  rd    op
		mask: 0b00000_11_00000_00000_000_00000_1111111,
		reqd: 0b00000_00_00000_00000_000_00000_1000111,
		name: "FMSUB.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      rs3      rs2   rs1   rm  rd    op
		mask: 0b00000_11_00000_00000_000_00000_1111111,
		reqd: 0b00000_00_00000_00000_000_00000_1001011,
		name: "FNMSUB.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      rs3      rs2   rs1   rm  rd    op
		mask: 0b00000_11_00000_00000_000_00000_1111111,
		reqd: 0b00000_00_00000_00000_000_00000_1001111,
		name: "FNMADD.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_000_00000_1111111,
		reqd: 0b0000000_00000_00000_000_00000_1010011,
		name: "FADD.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_000_00000_1111111,
		reqd: 0b0000100_00000_00000_000_00000_1010011,
		name: "FSUB.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_000_00000_1111111,
		reqd: 0b0001000_00000_00000_000_00000_1010011,
		name: "FMUL.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_000_00000_1111111,
		reqd: 0b0001100_00000_00000_000_00000_1010011,
		name: "FDIV.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_000_00000_1111111,
		reqd: 0b0101100_00000_00000_000_00000_1010011,
		name: "FSQRT.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0010000_00000_00000_000_00000_1010011,
		name: "FSGNJ.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0010000_00000_00000_001_00000_1010011,
		name: "FSGNJN.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0010000_00000_00000_010_00000_1010011,
		name: "FSGNJX.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0010100_00000_00000_000_00000_1010011,
		name: "FMIN.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0010100_00000_00000_001_00000_1010011,
		name: "FMAX.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_000_00000_1111111,
		reqd: 0b1100000_00000_00000_000_00000_1010011,
		name: "FCVT.W.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_000_00000_1111111,
		reqd: 0b1100000_00001_00000_000_00000_1010011,
		name: "FCVT.WU.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_111_00000_1111111,
		reqd: 0b1110000_00000_00000_000_00000_1010011,
		name: "FMV.X.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b1010000_00000_00000_010_00000_1010011,
		name: "FEQ.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b1010000_00000_00000_001_00000_1010011,
		name: "FLT.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b1010000_00000_00000_000_00000_1010011,
		name: "FLE.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_111_00000_1111111,
		reqd: 0b1110000_00000_00000_001_00000_1010011,
		name: "FCLASS.S",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_000_00000_1111111,
		reqd: 0b1101000_00000_00000_000_00000_1010011,
		name: "FCVT.S.W",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_000_00000_1111111,
		reqd: 0b1101000_00001_00000_000_00000_1010011,
		name: "FCVT.S.WU",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_111_00000_1111111,
		reqd: 0b1111000_00000_00000_000_00000_1010011,
		name: "FMV.W.X",
		extension: "RV32F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	// RV64F
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_000_00000_1111111,
		reqd: 0b1100000_00010_00000_000_00000_1010011,
		name: "FCVT.L.S",
		extension: "RV64F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_000_00000_1111111,
		reqd: 0b1100000_00011_00000_000_00000_1010011,
		name: "FCVT.LU.S",
		extension: "RV64F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_000_00000_1111111,
		reqd: 0b1101000_00010_00000_000_00000_1010011,
		name: "FCVT.S.L",
		extension: "RV64F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_000_00000_1111111,
		reqd: 0b1101000_00011_00000_000_00000_1010011,
		name: "FCVT.S.LU",
		extension: "RV64F",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	// RV32D
	Instruction {
		//      imm          rs1   fn3 rd    op
		mask: 0b000000000000_00000_111_00000_1111111,
		reqd: 0b000000000000_00000_011_00000_0000111,
		name: "FLD",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatI
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   fn3 imm   op
		mask: 0b0000000_00000_00000_111_00000_1111111,
		reqd: 0b0000000_00000_00000_011_00000_0100111,
		name: "FSD",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      rs3      rs2   rs1   rm  rd    op
		mask: 0b00000_11_00000_00000_000_00000_1111111,
		reqd: 0b00000_01_00000_00000_000_00000_1000011,
		name: "FMADD.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      rs3      rs2   rs1   rm  rd    op
		mask: 0b00000_11_00000_00000_000_00000_1111111,
		reqd: 0b00000_01_00000_00000_000_00000_1000111,
		name: "FMSUB.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      rs3      rs2   rs1   rm  rd    op
		mask: 0b00000_11_00000_00000_000_00000_1111111,
		reqd: 0b00000_01_00000_00000_000_00000_1001011,
		name: "FNMSUB.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      rs3      rs2   rs1   rm  rd    op
		mask: 0b00000_11_00000_00000_000_00000_1111111,
		reqd: 0b00000_01_00000_00000_000_00000_1001111,
		name: "FNMADD.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_000_00000_1111111,
		reqd: 0b0000001_00000_00000_000_00000_1010011,
		name: "FADD.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_000_00000_1111111,
		reqd: 0b0000101_00000_00000_000_00000_1010011,
		name: "FSUB.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_000_00000_1111111,
		reqd: 0b0001001_00000_00000_000_00000_1010011,
		name: "FMUL.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_000_00000_1111111,
		reqd: 0b0001101_00000_00000_000_00000_1010011,
		name: "FDIV.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_000_00000_1111111,
		reqd: 0b0101101_00000_00000_000_00000_1010011,
		name: "FSQRT.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0010001_00000_00000_000_00000_1010011,
		name: "FSGNJ.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0010001_00000_00000_001_00000_1010011,
		name: "FSGNJN.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0010001_00000_00000_010_00000_1010011,
		name: "FSGNJX.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0010101_00000_00000_000_00000_1010011,
		name: "FMIN.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b0010101_00000_00000_001_00000_1010011,
		name: "FMAX.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_000_00000_1111111,
		reqd: 0b0100000_00001_00000_000_00000_1010011,
		name: "FCVT.S.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_000_00000_1111111,
		reqd: 0b0100001_00000_00000_000_00000_1010011,
		name: "FCVT.D.S",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b1010001_00000_00000_010_00000_1010011,
		name: "FEQ.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b1010001_00000_00000_001_00000_1010011,
		name: "FLT.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_00000_00000_111_00000_1111111,
		reqd: 0b1010001_00000_00000_000_00000_1010011,
		name: "FLE.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_111_00000_1111111,
		reqd: 0b1110001_00000_00000_001_00000_1010011,
		name: "FCLASS.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_000_00000_1111111,
		reqd: 0b1100001_00000_00000_000_00000_1010011,
		name: "FCVT.W.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_000_00000_1111111,
		reqd: 0b1100001_00001_00000_000_00000_1010011,
		name: "FCVT.WU.D",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_000_00000_1111111,
		reqd: 0b1101001_00000_00000_000_00000_1010011,
		name: "FCVT.D.W",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_000_00000_1111111,
		reqd: 0b1101001_00001_00000_000_00000_1010011,
		name: "FCVT.D.WU",
		extension: "RV32D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	// RV64D
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_000_00000_1111111,
		reqd: 0b1100001_00010_00000_000_00000_1010011,
		name: "FCVT.L.D",
		extension: "RV64D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_000_00000_1111111,
		reqd: 0b1100001_00011_00000_000_00000_1010011,
		name: "FCVT.LU.D",
		extension: "RV64D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_111_00000_1111111,
		reqd: 0b1110001_00000_00000_000_00000_1010011,
		name: "FMV.X.D",
		extension: "RV64D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_000_00000_1111111,
		reqd: 0b1101001_00010_00000_000_00000_1010011,
		name: "FCVT.D.L",
		extension: "RV64D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_000_00000_1111111,
		reqd: 0b1101001_00011_00000_000_00000_1010011,
		name: "FCVT.D.LU",
		extension: "RV64D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	Instruction {
		//      imm     rs2   rs1   rm  rd    op
		mask: 0b1111111_11111_00000_111_00000_1111111,
		reqd: 0b1111001_00000_00000_000_00000_1010011,
		name: "FMV.D.X",
		extension: "RV64D",
		op: |cpu, word, _addr| {
			// FormatS
			Ok(())
		},
	},
	// Priviledged
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_11111_11111_111_11111_1111111,
		reqd: 0b0001000_00010_00000_000_00000_1110011,
		name: "SRET",
		extension: "Privileged",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	Instruction {
		//      fn7     rs2   rs1   fn3 rd    op
		mask: 0b1111111_11111_11111_111_11111_1111111,
		reqd: 0b0011000_00010_00000_000_00000_1110011,
		name: "MRET",
		extension: "Privileged",
		op: |cpu, word, _addr| {
			// FormatR
			Ok(())
		},
	},
	// TODO: remaining priviledged
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
#[ignore = "Takes long to run and maxes out the whole cpu. Only run when the \
            instructions change."]
fn unique_instruction_codes() {
	const THREADS: u32 = 8;
	const CHUNK: u32 = u32::MAX / THREADS as u32;

	let mut handles = Vec::with_capacity(THREADS as usize);

	for i in 0..THREADS {
		let start = i * CHUNK;
		let end = if i - 1 == THREADS { u32::MAX } else { (i + 1) * CHUNK };

		let handle = std::thread::spawn(move || {
			for word in start..=end {
				let mut found: Option<&Instruction> = None;

				for instr in &INSTRUCTIONS {
					if word & instr.mask == instr.reqd {
						if let Some(f) = found {
							panic!(
								"Found duplicate instruction code for `{}` \
								 and `{}`",
								instr.name, f.name
							);
						} else {
							found = Some(instr);
						}
					}
				}
			}
		});

		handles.push(handle);
	}

	for handle in handles {
		handle.join().unwrap();
	}
}
