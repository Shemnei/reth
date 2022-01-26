// Enable/disable `no_std` depending on the feature
#![cfg_attr(not(feature = "std"), no_std)]

mod util {
	/// Consumes bytes from the `bytes` slice and converts them
	/// to the requested type. The `bytes` slice will be advanced
	/// by the amount of bytes consumed for the type.
	macro_rules! consume {
		// Consumes a single byte from `bytes`.
		( $bytes:expr => u8 ) => {{
			consume!(@single $bytes)
		}};
		// Consumes `size_of<$type>` bytes and converts them to
		// `$type` with big endianness.
		( $bytes:expr => be $type:ty ) => {{
			const SIZE: usize = core::mem::size_of::<$type>();

			<$type>::f(consume!(@arr $bytes => SIZE))
		}};
		// Consumes `size_of<$type>` bytes and converts them to
		// `$type` with little endianness.
		( $bytes:expr => le $type:ty ) => {{
			const SIZE: usize = core::mem::size_of::<$type>();

			<$type>::from_le_bytes(consume!(@arr $bytes => SIZE))
		}};
		// Consumes `size_of<$type>` bytes and converts them to
		// the requested endianness (@see `EI_DATA_LE` and
		// `EI_DATA_BE` for possible values).
		( $bytes:expr , $endianness:expr => $type:ty ) => {{
			const SIZE: usize = core::mem::size_of::<$type>();

			// TODO: return err
			match $endianness {
				crate::header::consts::ident::data::EI_DATA_BE => Ok(<$type>::from_be_bytes(consume!(@arr $bytes => SIZE))),
				crate::header::consts::ident::data::EI_DATA_LE => Ok(<$type>::from_le_bytes(consume!(@arr $bytes => SIZE))),
				_ => Err(crate::error::Error::new(crate::error::ErrorKind::UnknownEndianess))
			}
		}};
		// Consumes `$len` bytes and returns them as array.
		( $bytes:expr => $len:expr ) => {{
			crate::util::consume!(@arr $bytes => $len)
		}};
		// PRIVATE: Shared code to consume a single byte.
		( @single $bytes:expr ) => {{
			let buf = $bytes[0];
			$bytes = &$bytes[1..];
			buf
		}};
		// PRIVATE: Shared code to consume a byte array.
		( @arr $bytes:expr => $len:expr ) => {{
			let mut buf = [0u8; $len];
			let (left, right) = $bytes.split_at($len);
			buf.copy_from_slice(left);
			$bytes = right;
			buf
		}};
	}

	/// Exports macro for use in other modules of the crate.
	pub(crate) use consume;

	/// Defines a list on constants with some added doc comments and a
	/// convienient function which converts a value of the shared `field/type`
	/// to a string representation.
	macro_rules! def_consts {
		(
			$field:ident : $size:ty : $as_str:ident => {
				$(
					$(
						#[doc = $doc:literal]
					)+
					$name:ident : $repr:literal = $value:literal ,
				)+
			}
			$(
				, {
					$(
						$extra_match_pattern:pat => $extra_match_value:literal ,
					)+
				}
			)?
		) => {
			$(
				#[doc = "Field `"]
				#[doc = stringify!($field)]
				#[doc = "`: "]
				$(
					#[doc = $doc]
				)+
				pub const $name: $size = $value;
			)+

			pub fn $as_str(value: $size) -> &'static str {
				match value {
					$(
						$name => $repr,
					)+
					$(
						$(
							$extra_match_pattern => $extra_match_value ,
						)+
					)?
					_ => "UNKNOWN",
				}
			}
		};
	}

	/// Exports macro for use in other modules of the crate.
	pub(crate) use def_consts;
}

pub mod error {
	use core::fmt;

	pub type Result<T, E = Error> = core::result::Result<T, E>;

	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
	pub enum ErrorKind {
		InsufficantSize,
		InvalidMagic,
		InvalidClass,
		UnknownEndianess,
	}

	impl fmt::Display for ErrorKind {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			match self {
				Self::InsufficantSize => {
					f.write_str("Not enough bytes found to parse")
				}
				Self::InvalidMagic => f.write_str(
					"Found invalid magic constant at the start of the header",
				),
				Self::InvalidClass => f.write_str(
					"Found invalid class in the elf header (expected 32 or \
					 64 bit)",
				),
				Self::UnknownEndianess => f.write_str(
					"Found unknown endianness in field `e_ident[EI_DATA]`",
				),
			}
		}
	}

	#[derive(Debug, Clone, PartialEq, Eq, Hash)]
	#[cfg_attr(not(feature = "std"), derive(Copy))]
	pub struct Error {
		pub kind: ErrorKind,
		#[cfg(feature = "std")]
		pub message: Option<String>,
	}

	impl Error {
		pub(crate) fn new(kind: ErrorKind) -> Self {
			Self {
				kind,
				#[cfg(feature = "std")]
				message: None,
			}
		}

		#[cfg(feature = "std")]
		pub(crate) fn with_message(
			mut self,
			message: impl fmt::Display,
		) -> Self {
			self.message = Some(message.to_string());
			self
		}
	}

	impl fmt::Display for Error {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			// TODO: print message
			fmt::Display::fmt(&self.kind, f)
		}
	}

	#[cfg(feature = "std")]
	impl std::error::Error for Error {}
}

pub mod header {
	/// # References
	/// - <https://refspecs.linuxbase.org/elf/elf.pdf>
	/// - <http://www.sco.com/developers/gabi/2000-07-17/ch4.eheader.html>
	/// - <https://en.wikipedia.org/wiki/Executable_and_Linkable_Format>
	pub mod consts {
		pub mod ident {
			pub mod index {
				/// Field `ei_mag0`: Magic constant `0x7f`.
				pub const EI_MAG0: usize = 0;

				/// Field `ei_mag1`: Magic constant `0x45`.
				pub const EI_MAG1: usize = 1;

				/// Field `ei_mag2`: Magic constant `0x4c`.
				pub const EI_MAG2: usize = 2;

				/// Field `ei_mag3`: Magic constant `0x46`.
				pub const EI_MAG3: usize = 3;

				/// Field `ei_class`: Signifies the 32- or 64-bit format.
				pub const EI_CLASS: usize = 4;

				/// Field `ei_data`: Signifies the little or big endianness.
				pub const EI_DATA: usize = 5;

				/// Field `ei_version`: Specifies the version of ELF.
				pub const EI_VERSION: usize = 6;

				/// Field `ei_osabi`: Identifies the target operating system abi.
				pub const EI_OSABI: usize = 7;

				/// Field `ei_abiversion`: Specifies the abi version.
				pub const EI_ABIVERSION: usize = 8;

				/// Field `ei_pad`: Start of unused padding.
				pub const EI_PAD_START: usize = 9;
			}

			pub mod class {
				crate::util::def_consts! {
					ei_class : u8 : ei_class_as_str => {
						/// Signifies 32-bit format.
						EI_CLASS_32 : "32-bit" = 1,

						/// Signifies 64-bit format.
						EI_CLASS_64 : "64-bit" = 2,
					}
				}
			}

			pub mod data {
				crate::util::def_consts! {
					ei_data : u8 : ei_data_as_str => {
						/// Signifies little endianness.
						EI_DATA_LE : "LE" = 1,

						/// Signifies big endianness.
						EI_DATA_BE : "BE" = 2,
					}
				}
			}

			pub mod version {
				/// Field `ei_version`: Original and current version.
				pub const EI_VERSION_CURRENT: u32 = 1;
			}

			pub mod osabi {
				crate::util::def_consts! {
					ei_osabi : u8 : ei_osabi_as_str => {
						/// System V.
						EI_OSABI_SYSTEMV : "System V" = 0x00,

						/// HP-UX.
						EI_OSABI_HPUX : "HP-UX" = 0x01,

						/// NetBSD.
						EI_OSABI_NETBSD : "NetBSD" = 0x02,

						/// Linux.
						EI_OSABI_LINUX : "Linux" = 0x03,

						/// GNU Hurd.
						EI_OSABI_GNUHURD : "GNU Hurd" = 0x04,

						/// Solaris.
						EI_OSABI_SOLARIS : "Solaris" = 0x06,

						/// AIX.
						EI_OSABI_AIX : "AIX" = 0x07,

						/// IRIX.
						EI_OSABI_IRIX : "IRIX" = 0x08,

						/// FreeBSD.
						EI_OSABI_FREEBSD : "FreeBSD" = 0x09,

						/// Tru64.
						EI_OSABI_TRU64 : "Tru64" = 0x0a,

						/// Novell Modesto.
						EI_OSABI_NOVELLMODESTO : "Novell Modesto" = 0x0b,

						/// OpenBSD.
						EI_OSABI_OPENBSD : "OpenBSD" = 0x0c,

						/// OpenVMS.
						EI_OSABI_OPENVMS : "OpenVMS" = 0x0d,

						/// NonStop Kernel.
						EI_OSABI_NONSTOPKERNEL : "NonStop Kernel" = 0x0e,

						/// AROS.
						EI_OSABI_AROS : "AROS" = 0x0f,

						/// Fenix OS.
						EI_OSABI_FENIXOS : "Fenix OS" = 0x10,

						/// CloudABI.
						EI_OSABI_CLOUDABI : "CloudABI" = 0x11,

						/// Stratus Technologies OpenVOS.
						EI_OSABI_OPENVOS : "Stratus Technologies OpenVOS" = 0x12,
					}
				}
			}
		}

		pub mod typ {
			crate::util::def_consts! {
				e_type : u16 : e_type_as_str => {
					/// ET_NONE.
					E_TYPE_ET_NONE : "ET_NONE" = 0x0000,

					/// ET_REL.
					E_TYPE_ET_REL : "ET_REL" = 0x0001,

					/// ET_EXEC.
					E_TYPE_ET_EXEC : "ET_EXEC" = 0x0002,

					/// ET_DYN.
					E_TYPE_ET_DYN : "ET_DYN" = 0x0003,

					/// ET_CORE.
					E_TYPE_ET_CORE : "ET_CORE" = 0x0004,

					/// ET_LOOS.
					E_TYPE_ET_LOOS : "ET_LOOS" = 0xfe00,

					/// ET_HIOS.
					E_TYPE_ET_HIOS : "ET_HIOS" = 0xfeff,

					/// ET_LOPROC.
					E_TYPE_ET_LOPROC : "ET_LOPROC" = 0xff00,

					/// ET_HIPROC.
					E_TYPE_ET_HIPROC : "ET_HIPROC" = 0xffff,
				}
			}
		}

		pub mod machine {
			crate::util::def_consts! {
				e_machine : u16 : e_machine_as_str => {
					/// Unspecified.
					E_MACHINE_UNSPECIFIED : "Unspecified" = 0x0000,

					/// AT&T WE 32100.
					E_MACHINE_ATTWE32100 : "AT&T WE 32100" = 0x0001,

					/// SPARC.
					E_MACHINE_SPARC : "SPARC" = 0x0002,

					/// x86.
					E_MACHINE_X86 : "x86" = 0x0003,

					/// Motorola 68000 (M68k).
					E_MACHINE_MOTOROLA68000 : "Motorola 68000 (M68k)" = 0x0004,

					/// Motorola 88000 (M88k).
					E_MACHINE_MOTOROLA88000 : "Motorola 88000 (M88k)" = 0x0005,

					/// Intel MCU.
					E_MACHINE_INTELMCU : "Intel MCU" = 0x0006,

					/// Intel 80860.
					E_MACHINE_INTEL80860 : "Intel 80860" = 0x0007,

					/// MIPS.
					E_MACHINE_MIPS : "MIPS" = 0x0008,

					/// IBM System/370.
					E_MACHINE_IBM370 : "IBM System/370" = 0x0009,

					/// MIPS RS3000 Little-endian.
					E_MACHINE_MIPSRS3000LE : "MIPS RS3000 Little-endian" = 0x000a,

					/// Hewlett-Packard PA-RISC.
					E_MACHINE_HPPARISC : "Hewlett-Packard PA-RISC" = 0x000e,

					/// Intel 80960.
					E_MACHINE_INTEL80960 : "Intel 80960" = 0x0013,

					/// PowerPC.
					E_MACHINE_POWERPC : "PowerPC" = 0x0014,

					/// PowerPC (64-bit).
					E_MACHINE_POWERPC64 : "PowerPC (64-bit)" = 0x0015,

					/// S390, including S390x.
					E_MACHINE_S390 : "S390, including S390x" = 0x0016,

					/// IBM SPU/SPC.
					E_MACHINE_IBMSPUSPC : "IBM SPU/SPC" = 0x0017,

					/// NEC V800.
					E_MACHINE_NECV800 : "NEC V800" = 0x0024,

					/// Fujitsu FR20.
					E_MACHINE_FUJITSUFR20 : "Fujitsu FR20" = 0x0025,

					/// TRW RH-32.
					E_MACHINE_TRWRH32 : "TRW RH-32" = 0x0026,

					/// Motorola RCE.
					E_MACHINE_MOTOROLARCE : "Motorola RCE" = 0x0027,

					/// ARM (up to ARMv7/Aarch32).
					E_MACHINE_ARM : "ARM (up to ARMv7/Aarch32)" = 0x0028,

					/// Digital Alpha.
					E_MACHINE_DIGITALALPHA : "Digital Alpha" = 0x0029,

					/// SuperH.
					E_MACHINE_SUPERH : "SuperH" = 0x002a,

					/// SPARC Version 9.
					E_MACHINE_SPARC9 : "SPARC Version 9" = 0x002b,

					/// Siemens TriCore embedded processor.
					E_MACHINE_SIEMENSTRICORE : "Siemens TriCore embedded processor" = 0x002c,

					/// Argonaut RISC Core.
					E_MACHINE_ARGONAUTRISCCORE : "Argonaut RISC Core" = 0x002d,

					/// Hitachi H8/300.
					E_MACHINE_HITACHIH8300 : "Hitachi H8/300" = 0x002e,

					/// Hitachi H8/300H.
					E_MACHINE_HITACHIH8300H : "Hitachi H8/300H" = 0x002f,

					/// Hitachi H8S.
					E_MACHINE_HITACHIH8S : "Hitachi H8S" = 0x0030,

					/// Hitachi H8/500.
					E_MACHINE_HITACHIH8500 : "Hitachi H8/500" = 0x0031,

					/// IA-64.
					E_MACHINE_IA64 : "IA-64" = 0x0032,

					/// Stanford MIPS-X.
					E_MACHINE_STANFORDMIPSX : "Stanford MIPS-X" = 0x0033,

					/// Motorola ColdFire.
					E_MACHINE_MOTOROLACOLDFIRE : "Motorola ColdFire" = 0x0034,

					/// Motorola M68HC12.
					E_MACHINE_MOTOROLAM68HC12 : "Motorola M68HC12" = 0x0035,

					/// Fujitsu MMA Multimedia Accelerator.
					E_MACHINE_FUJITSUMMA : "Fujitsu MMA Multimedia Accelerator" = 0x0036,

					/// Siemens PCP.
					E_MACHINE_SIEMENSPCP : "Siemens PCP" = 0x0037,

					/// Sony nCPU embedded RISC processor.
					E_MACHINE_SONYNCPURISC : "Sony nCPU embedded RISC processor" = 0x0038,

					/// Denso NDR1 microprocessor.
					E_MACHINE_DENSONDR1 : "Denso NDR1 microprocessor" = 0x0039,

					/// Motorola Star*Core processor.
					E_MACHINE_MOTOROLASTARCORE : "Motorola Star*Core processor" = 0x003a,

					/// Toyota ME16 processor.
					E_MACHINE_TOYOTAME16 : "Toyota ME16 processor" = 0x003b,

					/// STMicroelectronics ST100 processor.
					E_MACHINE_STMST100 : "STMicroelectronics ST100 processor" = 0x003c,

					/// Advanced Logic Corp. Tinyj embedded processor family.
					E_MACHINE_ALCTINYJ : "Advanced Logic Corp. Tinyj embedded processor family" = 0x003d,

					/// AMD x86-64.
					E_MACHINE_AMD8664 : "AMD x86-64" = 0x003e,

					/// TMS320C6000 Family.
					E_MACHINE_TMS320C6000 : "TMS320C6000 Family" = 0x008c,

					/// MCST Elbrus e2k.
					E_MACHINE_MCSTELBRUSE2K : "MCST Elbrus e2k" = 0x00af,

					/// ARM 64-bits (ARMv8/Aarch64).
					E_MACHINE_ARM64 : "ARM 64-bits (ARMv8/Aarch64)" = 0x00b7,

					/// RISC-V.
					E_MACHINE_RISCV : "RISC-V" = 0x00f3,

					/// Berkeley Packet Filter.
					E_MACHINE_BPF : "Berkeley Packet Filter" = 0x00f7,

					/// WDC 65C816.
					E_MACHINE_WDC65C816 : "WDC 65C816" = 0x0101,
				}, {
					(0x0b..=0x0d) => "RESERVED",
					(0x18..=0x23) => "RESERVED",
				}
			}
		}
	}

	use core::fmt;

	#[repr(C)]
	#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
	pub struct Ident(pub [u8; 16]);

	impl Ident {
		pub fn ei_mag(&self) -> &[u8] {
			&self.0[..consts::ident::index::EI_CLASS]
		}

		pub fn ei_class(&self) -> u8 {
			self.0[consts::ident::index::EI_CLASS]
		}

		pub fn ei_data(&self) -> u8 {
			self.0[consts::ident::index::EI_DATA]
		}

		pub fn ei_version(&self) -> u8 {
			self.0[consts::ident::index::EI_VERSION]
		}

		pub fn ei_osabi(&self) -> u8 {
			self.0[consts::ident::index::EI_OSABI]
		}

		pub fn ei_abiversion(&self) -> u8 {
			self.0[consts::ident::index::EI_ABIVERSION]
		}

		pub fn ei_pad(&self) -> &[u8] {
			&self.0[consts::ident::index::EI_PAD_START..]
		}
	}

	#[rustfmt::skip]
	impl fmt::Display for Ident {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			f.write_fmt(format_args!(r#"Ident:
	ei_class     : {}
	ei_data      : {}
	ei_version   : {}
	ei_osabi     : {}
	ei_abiversion: {}"#,
				crate::header::consts::ident::class::ei_class_as_str(self.ei_class()),
				crate::header::consts::ident::data::ei_data_as_str(self.ei_data()),
				self.ei_version(),
				crate::header::consts::ident::osabi::ei_osabi_as_str(self.ei_osabi()),
				self.ei_abiversion()
			))
		}
	}

	macro_rules! header {
		( $size:ty ) => {
			#[repr(C)]
			#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
			pub struct Header {
				/// Field `e_ident`: Identifiers.
				pub e_ident: crate::header::Ident,

				/// Field `e_type`: Identifies object file type.
				pub e_type: u16,

				/// Field `e_machine`: Specifies a target instruction set
				/// architecture.
				pub e_machine: u16,

				/// Field `e_version`: Specifies the version of elf
				/// (`1` for the original version).
				pub e_version: u32,

				/// Field `e_entry`: Memory address of the entry point.
				pub e_entry: $size,

				/// Field `e_phoff`: Points to the start of the program header
				/// table.
				pub e_phoff: $size,

				/// Field `e_shoff`: Points to the start of the section header
				/// table.
				pub e_shoff: $size,

				/// Field `e_flags`: Flags (interpretation of the field depends
				/// on the target architecture).
				pub e_flags: u32,

				/// Field `e_ehsize`: Contains the size of this [`Header`].
				pub e_ehsize: u16,

				/// Field `e_phentsize`: Contains the size of a program header
				/// table entry.
				pub e_phentsize: u16,

				/// Field `e_phnum`: Contains the number of entries in the
				/// program header table.
				pub e_phnum: u16,

				/// Field `e_shentsize`: Contains the size of a section header
				/// table entry.
				pub e_shentsize: u16,

				/// Field `e_shnum`: Contains the number of entries in the
				/// section header table.
				pub e_shnum: u16,

				/// Field `e_shstrndx`: Contains index of the section header
				/// table entry that contains the section names.
				pub e_shstrndx: u16,
			}

			impl Header {
                #[allow(unused_assignments, clippy::eval_order_dependence)]
				pub fn from_bytes(mut bytes: &[u8]) -> crate::error::Result<Self> {
					use crate::util::consume;

					if bytes.len() < core::mem::size_of::<Self>() {
                        return Err(crate::error::Error::new(crate::error::ErrorKind::InsufficantSize))
					}

                    const SIZE_IDENT: usize = core::mem::size_of::<crate::header::Ident>();

                    let e_ident = crate::header::Ident(consume!(bytes => SIZE_IDENT));
                    if e_ident.ei_mag() != &[0x7f, 0x45, 0x4c, 0x46] {
                        return Err(crate::error::Error::new(crate::error::ErrorKind::InvalidMagic))
                    }
                    let endianness = e_ident.ei_data();

                    Ok(Self {
                        e_ident,
                        e_type: consume!(bytes, endianness => u16)?,
                        e_machine: consume!(bytes, endianness => u16)?,
                        e_version: consume!(bytes, endianness => u32)?,
                        e_entry: consume!(bytes, endianness => $size)?,
                        e_phoff: consume!(bytes, endianness => $size)?,
                        e_shoff: consume!(bytes, endianness => $size)?,
                        e_flags: consume!(bytes, endianness => u32)?,
                        e_ehsize: consume!(bytes, endianness => u16)?,
                        e_phentsize: consume!(bytes, endianness => u16)?,
                        e_phnum: consume!(bytes, endianness => u16)?,
                        e_shentsize: consume!(bytes, endianness => u16)?,
                        e_shnum: consume!(bytes, endianness => u16)?,
                        e_shstrndx: consume!(bytes, endianness => u16)?,
                    })
				}
			}

			impl core::fmt::Display for Header {
				fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
					f.write_fmt(format_args!(r#"Header:
	Ident:
		ei_class     : {}
		ei_data      : {}
		ei_version   : {}
		ei_osabi     : {}
		ei_abiversion: {}
	e_type     : {}
	e_machine  : {}
	e_version  : {}
	e_entry    : 0x{:0size$x}
	e_phoff    : {}
	e_shoff    : {}
	e_flags    : 0b{:032b}
	e_ehsize   : {}
	e_phentsize: {}
	e_phnum    : {}
	e_shentsize: {}
	e_shnum    : {}
	e_shstrndx : {}"#,
						crate::header::consts::ident::class::ei_class_as_str(self.e_ident.ei_class()),
						crate::header::consts::ident::data::ei_data_as_str(self.e_ident.ei_data()),
						self.e_ident.ei_version(),
						crate::header::consts::ident::osabi::ei_osabi_as_str(self.e_ident.ei_osabi()),
						self.e_ident.ei_abiversion(),
						crate::header::consts::typ::e_type_as_str(self.e_type),
						crate::header::consts::machine::e_machine_as_str(self.e_machine),
						self.e_version,
						self.e_entry,
						self.e_phoff,
						self.e_shoff,
						self.e_flags,
						self.e_ehsize,
						self.e_phentsize,
						self.e_phnum,
						self.e_shentsize,
						self.e_shnum,
						self.e_shstrndx,
						size = core::mem::size_of::<$size>() * 2,
					))
				}
			}
		};
	}

	pub mod elf32 {
		header!(u32);
	}

	pub mod elf64 {
		header!(u64);

		#[cfg(test)]
		mod tests {

			#[test]
			#[cfg(feature = "std")]
			fn punktf_header() {
				use super::*;

				let bytes = [
					0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01, 0x00, 0x00,
					0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00,
					0x3e, 0x00, 0x01, 0x00, 0x00, 0x00, 0x80, 0x98, 0x07,
					0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00,
					0x00, 0x00, 0x00, 0x00, 0x38, 0xb8, 0x3d, 0x00, 0x00,
					0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00,
					0x38, 0x00, 0x0c, 0x00, 0x40, 0x00, 0x2b, 0x00, 0x29,
					0x00,
				];

				println!("Should: {}", std::mem::size_of::<Header>());
				println!("Is    : {}", bytes.len());

				let header = Header::from_bytes(&bytes).unwrap();
				println!("{:#}", header);
			}
		}
	}
}

pub mod program_header {
	pub mod consts {
		pub mod typ {
			crate::util::def_consts! {
				p_type : u32 : p_type_as_str => {
					/// Program header table entry unused.
					P_TYPE_PT_NULL : "PT_NULL" = 0x00000000,

					/// Loadable segment.
					P_TYPE_PT_LOAD: "PT_LOAD" = 0x00000001,

					/// Dynamic linking information.
					P_TYPE_PT_DYNAMIC: "PT_DYNAMIC" = 0x00000002,

					/// Interpreter information.
					P_TYPE_PT_INTERP: "PT_INTERP" = 0x00000003,

					/// Auxiliary information.
					P_TYPE_PT_NOTE: "PT_NOTE" = 0x00000004,

					/// Reserved.
					P_TYPE_PT_SHLIB: "PT_SHLIB" = 0x00000005,

					/// Segment containing program header table itself.
					P_TYPE_PT_PHDR: "PT_PHDR" = 0x00000006,

					/// Thead-Local Storage template.
					P_TYPE_PT_TLS: "PT_TLS" = 0x00000007,
				}, {
					(0x60000000..=0x6FFFFFFF) => "RESERVED: Operating system specific",
					(0x70000000..=0x7FFFFFFF) => "RESERVED: Processor specific",
				}
			}
		}
	}

	/// # Note
	///
	/// There is no simple way to generate the headers for this module via a
	/// macro as the field `p_flags` has a different position depending on the
	/// bitness of the elf.

	pub mod elf32 {
		use core::fmt;

		use crate::error::Result;

		#[repr(C)]
		#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
		pub struct ProgramHeader {
			/// Field `p_type`: Identifies the type of the segment.
			pub p_type: u32,

			/// Field `p_offset`: Offset of the segment in the file image.
			pub p_offset: u32,

			/// Field `p_vaddr`: Virtual address of the segment in memory.
			pub p_vaddr: u32,

			/// Field `p_paddr`: On systems where physical address is relevant,
			/// reserved for segment's physical address.
			pub p_paddr: u32,

			/// Field `p_filesz`: Size in bytes of the segment in the file image
			/// (may be 0).
			pub p_filesz: u32,

			/// Field `p_memsz`: Size in bytes of the segment in memory
			/// (may be 0).
			pub p_memsz: u32,

			/// Field `p_flags`: Segment-dependent flags.
			pub p_flags: u32,

			/// Field `p_align`: Specifies alignment.
			///
			/// `0` and `1` specify no alignment. Otherwise should be a positive,
			/// integral power of `2` with `p_vaddr` equating `p_offset` modulus
			/// `p_align`.
			pub p_align: u32,
		}

		impl ProgramHeader {
			#[allow(unused_assignments, clippy::eval_order_dependence)]
			pub fn from_bytes(
				endianness: u8,
				mut bytes: &[u8],
			) -> Result<Self> {
				use crate::util::consume;

				Ok(Self {
					p_type: consume!(bytes, endianness => u32)?,
					p_offset: consume!(bytes, endianness => u32)?,
					p_vaddr: consume!(bytes, endianness => u32)?,
					p_paddr: consume!(bytes, endianness => u32)?,
					p_filesz: consume!(bytes, endianness => u32)?,
					p_memsz: consume!(bytes, endianness => u32)?,
					p_flags: consume!(bytes, endianness => u32)?,
					p_align: consume!(bytes, endianness => u32)?,
				})
			}

			pub fn extract_data<'a>(&self, bytes: &'a [u8]) -> &'a [u8] {
				let start = self.p_offset as usize;
				let end = start + (self.p_filesz as usize);

				core::ops::Index::index(bytes, start..end)
			}
		}

		impl core::ops::Index<&ProgramHeader> for &[u8] {
			type Output = [u8];

			fn index(&self, index: &ProgramHeader) -> &Self::Output {
				index.extract_data(self)
			}
		}

		#[rustfmt::skip]
		impl fmt::Display for ProgramHeader {
			fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
				f.write_fmt(format_args!(r#"ProgramHeader:
	p_type  : {}
	p_offset: {}
	p_vaddr : 0x{:08x}
	p_paddr : 0x{:08x}
	p_filesz: {}
	p_memsz : {}
	p_flags : 0b{:032b}
	p_align : {}"#,
					crate::program_header::consts::typ::p_type_as_str(self.p_type),
					self.p_flags,
					self.p_offset,
					self.p_vaddr,
					self.p_paddr,
					self.p_filesz,
					self.p_memsz,
					self.p_align
				))
			}
		}
	}

	pub mod elf64 {
		use core::fmt;

		use crate::error::Result;

		#[repr(C)]
		#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
		pub struct ProgramHeader {
			/// Field `p_type`: Identifies the type of the segment.
			pub p_type: u32,

			/// Field `p_flags`: Segment-dependent flags.
			pub p_flags: u32,

			/// Field `p_offset`: Offset of the segment in the file image.
			pub p_offset: u64,

			/// Field `p_vaddr`: Virtual address of the segment in memory.
			pub p_vaddr: u64,

			/// Field `p_paddr`: On systems where physical address is relevant,
			/// reserved for segment's physical address.
			pub p_paddr: u64,

			/// Field `p_filesz`: Size in bytes of the segment in the file image
			/// (may be 0).
			pub p_filesz: u64,

			/// Field `p_memsz`: Size in bytes of the segment in memory
			/// (may be 0).
			pub p_memsz: u64,

			/// Field `p_align`: Specifies alignment.
			///
			/// `0` and `1` specify no alignment. Otherwise should be a positive,
			/// integral power of `2` with `p_vaddr` equating `p_offset` modulus
			/// `p_align`.
			pub p_align: u64,
		}

		impl ProgramHeader {
			#[allow(unused_assignments, clippy::eval_order_dependence)]
			pub fn from_bytes(
				endianness: u8,
				mut bytes: &[u8],
			) -> Result<Self> {
				use crate::util::consume;

				Ok(Self {
					p_type: consume!(bytes, endianness => u32)?,
					p_flags: consume!(bytes, endianness => u32)?,
					p_offset: consume!(bytes, endianness => u64)?,
					p_vaddr: consume!(bytes, endianness => u64)?,
					p_paddr: consume!(bytes, endianness => u64)?,
					p_filesz: consume!(bytes, endianness => u64)?,
					p_memsz: consume!(bytes, endianness => u64)?,
					p_align: consume!(bytes, endianness => u64)?,
				})
			}

			pub fn extract_data<'a>(&self, bytes: &'a [u8]) -> &'a [u8] {
				let start = self.p_offset as usize;
				let end = start + (self.p_filesz as usize);

				core::ops::Index::index(bytes, start..end)
			}
		}

		impl core::ops::Index<&ProgramHeader> for &[u8] {
			type Output = [u8];

			fn index(&self, index: &ProgramHeader) -> &Self::Output {
				index.extract_data(self)
			}
		}

		#[rustfmt::skip]
		impl fmt::Display for ProgramHeader {
			fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
				f.write_fmt(format_args!(r#"ProgramHeader:
	p_type  : {}
	p_flags : 0b{:032b}
	p_offset: {}
	p_vaddr : 0x{:016x}
	p_paddr : 0x{:016x}
	p_filesz: {}
	p_memsz : {}
	p_align : {}"#,
					crate::program_header::consts::typ::p_type_as_str(self.p_type),
					self.p_flags,
					self.p_offset,
					self.p_vaddr,
					self.p_paddr,
					self.p_filesz,
					self.p_memsz,
					self.p_align
				))
			}
		}
	}
}

pub mod section_header {
	pub mod consts {
		pub mod typ {
			crate::util::def_consts! {
				sh_type : u32 : sh_type_as_str => {
					/// Section header table entry unused.
					SH_TYPE_SHT_NULL: "SHT_NULL" = 0x00000000,

					/// Program data.
					SH_TYPE_SHT_PROGBITS: "SHT_PROGBITS" = 0x00000001,

					/// Symbol table.
					SH_TYPE_SHT_SYMTAB: "SHT_SYMTAB" = 0x00000002,

					/// String table.
					SH_TYPE_SHT_STRTAB: "SHT_STRTAB" = 0x00000003,

					/// Relocation entries with addends.
					SH_TYPE_SHT_RELA: "SHT_RELA" = 0x00000004,

					/// Symbol hash table.
					SH_TYPE_SHT_HASH: "SHT_HASH" = 0x00000005,

					/// Dynamic linking information.
					SH_TYPE_SHT_DYNAMIC: "SHT_DYNAMIC" = 0x00000006,

					/// Notes.
					SH_TYPE_SHT_NOTE: "SHT_NOTE" = 0x00000007,

					/// Program space with no data (bss).
					SH_TYPE_SHT_NOBITS: "SHT_NOBITS" = 0x00000008,

					/// Relocation entries, no addends.
					SH_TYPE_SHT_REL: "SHT_REL" = 0x00000009,

					/// Reserved.
					SH_TYPE_SHT_SHLIB: "SHT_SHLIB" = 0x0000000a,

					/// Dynamic linker symbol table.
					SH_TYPE_SHT_DYNSYM: "SHT_DYNSYM" = 0x0000000b,

					/// Array of constructors.
					SH_TYPE_SHT_INIT_ARRAY: "SHT_INIT_ARRAY" = 0x0000000e,

					/// Array of destructors.
					SH_TYPE_SHT_FINI_ARRAY: "SHT_FINI_ARRAY" = 0x0000000f,

					/// Array of pre-constructors.
					SH_TYPE_SHT_PREINIT_ARRAY: "SHT_PREINIT_ARRAY" = 0x00000010,

					/// Section group.
					SH_TYPE_SHT_GROUP: "SHT_GROUP" = 0x00000011,

					/// Extended section indices.
					SH_TYPE_SHT_SYMTAB_SHNDX: "SHT_SYMTAB_SHNDX" = 0x00000012,

					/// Number of defined types.
					SH_TYPE_SHT_NUM: "SHT_NUM" = 0x00000013,
				}, {
					(0x60000000..) => "RESERVED: Operating system specific",
				}
			}
		}

		pub mod flags {
			macro_rules! def_flags {
				( $size:ty ) => {
					/// Field `sh_flags`: Writable.
					pub const SH_FLAG_SHF_WRITE: $size = 0x01;

					/// Field `sh_flags`: Occupies memory during execution.
					pub const SH_FLAG_SHF_ALLOC: $size = 0x02;

					/// Field `sh_flags`: Executable.
					pub const SH_FLAG_SHF_EXECINSTR: $size = 0x03;

					/// Field `sh_flags`: Might be merged.
					pub const SH_FLAG_SHF_MERGE: $size = 0x10;

					/// Field `sh_flags`: Contains null-terminated strings.
					pub const SH_FLAG_SHF_STRINGS: $size = 0x20;

					/// Field `sh_flags`: `sh_info` contains SHT index.
					pub const SH_FLAG_SHF_INFO_LINK: $size = 0x40;

					/// Field `sh_flags`: Preserved order after combining.
					pub const SH_FLAG_SHF_LINK_ORDER: $size = 0x80;

					/// Field `sh_flags`: Non-standard OS specific handling
					/// required.
					pub const SH_FLAG_SHF_OS_NONCONFORMING: $size = 0x100;

					/// Field `sh_flags`: Section is member of a group.
					pub const SH_FLAG_SHF_GROUP: $size = 0x200;

					/// Field `sh_flags`: Section holds thread-local data.
					pub const SH_FLAG_SHF_TLS: $size = 0x400;

					/// Field `sh_flags`: OS-specific (mask).
					pub const SH_FLAG_SHF_MASKOS: $size = 0x0ff0_0000;

					/// Field `sh_flags`: Processor-specific (mask).
					pub const SH_FLAG_SHF_MASKPROC: $size = 0xf000_0000;

					/// Field `sh_flags`: Special ordering requirement (Solaris).
					pub const SH_FLAG_SHF_ORDERED: $size = 0x400_0000;

					/// Field `sh_flags`: Section is excluded unless referenced
					/// or allocated (Solaris).
					pub const SH_FLAG_SHF_EXCLUDE: $size = 0x800_0000;
				};
			}

			pub mod elf32 {
				def_flags!(u32);
			}

			pub mod elf64 {
				def_flags!(u64);
			}
		}
	}

	macro_rules! section_header {
		( $size:ty ) => {
			#[repr(C)]
			#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
			pub struct SectionHeader {
				/// Field `sh_name`: Offset to a string in the `.shstrtab`
				/// section containing the name of the section.
				pub sh_name: u32,

				/// Field `sh_type`: Identifies the type of this header.
				pub sh_type: u32,

				/// Field `sh_flags`: Identifies the attributes of the section.
				pub sh_flags: $size,

				/// Field `sh_addr`: Virtual address of the section in memory,
				/// for sections that are loaded.
				pub sh_addr: $size,

				/// Field `sh_offset`: Offset of the section in the file image.
				pub sh_offset: $size,

				/// Field `sh_size`: Size in bytes of the section in the file
				/// image (may be 0).
				pub sh_size: $size,

				/// Field `sh_link`: Contains the section index of an associated
				/// section.
				///
				/// This field has several purposes, depending on the
				/// type of the section.
				pub sh_link: u32,

				/// Field `sh_info`: Contains extra information about the section.
				///
				/// This field has several purposes, depending on the
				/// type of the section.
				pub sh_info: u32,

				/// Field `sh_addralign`: Contains the required alignment of the
				/// section.
				///
				/// The field must be a power of `2`.
				pub sh_addralign: $size,

				/// Field `sh_entsize`: Size in bytes of each entry.
				///
				/// This is only used when the entries are of a fixed-size.
				/// Otherwise the field contains `0`.
				pub sh_entsize: $size,
			}

			impl SectionHeader {
                #[allow(unused_assignments, clippy::eval_order_dependence)]
				pub fn from_bytes(endianness: u8, mut bytes: &[u8]) -> crate::error::Result<Self> {
					use crate::util::consume;

                    Ok(Self {
                        sh_name: consume!(bytes, endianness => u32)?,
                        sh_type: consume!(bytes, endianness => u32)?,
                        sh_flags: consume!(bytes, endianness => $size)?,
                        sh_addr: consume!(bytes, endianness => $size)?,
                        sh_offset: consume!(bytes, endianness => $size)?,
                        sh_size: consume!(bytes, endianness => $size)?,
                        sh_link: consume!(bytes, endianness => u32)?,
                        sh_info: consume!(bytes, endianness => u32)?,
                        sh_addralign: consume!(bytes, endianness => $size)?,
                        sh_entsize: consume!(bytes, endianness => $size)?,
                    })
				}

				pub fn extract_data<'a>(&self, bytes: &'a[u8]) -> &'a [u8] {
					let start = self.sh_offset as usize;
					let end = start + (self.sh_size as usize);

					core::ops::Index::index(bytes, start..end)
				}
			}

			impl core::ops::Index<&SectionHeader> for &[u8] {
				type Output = [u8];

				fn index(&self, index: &SectionHeader) -> &Self::Output {
					index.extract_data(self)
				}
   			}

			impl core::fmt::Display for SectionHeader {
				fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
					f.write_fmt(format_args!(r#"SectionHeader:
	sh_name     : {}
	sh_type     : {}
	sh_flags    : 0b{:0size_bin$b}
	sh_addr     : 0x{:0size_hex$x}
	sh_offset   : {}
	sh_size     : {}
	sh_link     : {}
	sh_info     : {}
	sh_addralign: {}
	sh_entsize  : {}"#,
						self.sh_name,
						crate::section_header::consts::typ::sh_type_as_str(self.sh_type),
						self.sh_flags,
						self.sh_addr,
						self.sh_offset,
						self.sh_size,
						self.sh_link,
						self.sh_info,
						self.sh_addralign,
						self.sh_entsize,
						size_hex = core::mem::size_of::<$size>() * 2,
						size_bin = core::mem::size_of::<$size>() * 8,
					))
				}
			}
		}
	}

	pub mod elf32 {
		section_header!(u32);
	}

	pub mod elf64 {
		section_header!(u64);
	}
}

pub mod strtab {
	pub struct Strtab<'a> {
		delim: u8,
		data: &'a [u8],
	}

	impl<'a> Strtab<'a> {
		pub const DEFAULT_DELIM: u8 = b'\0';

		pub fn new(delim: u8, data: &'a [u8]) -> Self {
			Self { data, delim }
		}

		pub fn get_bytes(
			&self,
			index: usize,
		) -> core::option::Option<&'a [u8]> {
			self.data.split(|b| b == &self.delim).skip(index).next()
		}

		pub fn get_bytes_off(
			&self,
			offset: usize,
		) -> core::option::Option<&'a [u8]> {
			let data = core::ops::Index::index(self.data, offset..);
			data.split(|b| b == &self.delim).next()
		}

		pub unsafe fn get_str(
			&self,
			index: usize,
		) -> core::option::Option<
			core::result::Result<&'a str, core::str::Utf8Error>,
		> {
			self.get_bytes(index).map(|b| core::str::from_utf8(b))
		}

		pub unsafe fn get_str_unchecked(
			&self,
			index: usize,
		) -> core::option::Option<&'a str> {
			self.get_bytes(index).map(|b| core::str::from_utf8_unchecked(b))
		}

		pub unsafe fn get_str_off_unchecked(
			&self,
			offset: usize,
		) -> core::option::Option<&'a str> {
			self.get_bytes_off(offset)
				.map(|b| core::str::from_utf8_unchecked(b))
		}
	}
}

pub mod symtab {
	macro_rules! symbol_table {
		( $size:ty ) => {
			#[repr(C)]
			#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
			pub struct Symbol {
				/// Field `st_name`: Name of the symbol.
				pub st_name: u32,

				/// Field `st_value`: Value of the symbol.
				pub st_value: $size,

				/// Field `st_size`: Size of the symbol.
				pub st_size: $size,

				/// Field `st_info`: Additional information.
				pub st_info: u8,

				/// Field `st_other`: Other (currently not used).
				pub st_other: u8,

				/// Field `st_shndx`: Index of the SectionHeader.
				pub st_shndx: u16,
			}

			impl Symbol {
                #[allow(unused_assignments, clippy::eval_order_dependence)]
				pub fn from_bytes(endianness: u8, mut bytes: &[u8]) -> crate::error::Result<Self> {
					use crate::util::consume;

                    Ok(Self {
                        st_name: consume!(bytes, endianness => u32)?,
                        st_value: consume!(bytes, endianness => $size)?,
                        st_size: consume!(bytes, endianness => $size)?,
                        st_info: consume!(bytes, endianness => u8)?,
                        st_other: consume!(bytes, endianness => u8)?,
                        st_shndx: consume!(bytes, endianness => u16)?,
                    })
				}
			}

			impl core::fmt::Display for Symbol {
				fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
					f.write_fmt(format_args!(r#"Symbol:
	st_name : {}
	st_value: {}
	st_size : {}
	st_info : {}
	st_other: {}
	st_shndx: {}"#,
						self.st_name,
						self.st_value,
						self.st_size,
						self.st_info,
						self.st_other,
						self.st_shndx,
					))
				}
			}

			pub struct Symtab<'a> {
				endianness: u8,
				data: &'a [u8],
			}

			impl<'a> Symtab<'a> {
				const SYMBOL_SIZE: usize = core::mem::size_of::<Symbol>();

				pub fn new(endianness: u8, data: &'a [u8]) -> Self {
					Self { endianness, data }
				}

				pub fn len(&self) -> usize {
					self.data.len() / Self::SYMBOL_SIZE
				}

				pub fn get_symbol(
					&self,
					index: usize,
				) -> core::option::Option<Symbol> {
					let start = index * Self::SYMBOL_SIZE;

					if start < self.data.len() {
						let data = core::ops::Index::index(self.data, start..);
						Symbol::from_bytes(self.endianness, data).ok()
					} else {
						None
					}
				}
			}
		};
	}

	pub mod elf32 {
		symbol_table!(u32);
	}

	pub mod elf64 {
		symbol_table!(u64);
	}
}

#[cfg(feature = "std")]
pub mod elf {
	use crate::error::{Error, ErrorKind, Result};
	use crate::header::consts::ident::class::{EI_CLASS_32, EI_CLASS_64};
	use crate::header::consts::ident::index::EI_CLASS;
	use crate::header::elf32::Header as Header32;
	use crate::header::elf64::Header as Header64;
	use crate::program_header::elf32::ProgramHeader as ProgramHeader32;
	use crate::program_header::elf64::ProgramHeader as ProgramHeader64;
	use crate::section_header::elf32::SectionHeader as SectionHeader32;
	use crate::section_header::elf64::SectionHeader as SectionHeader64;

	pub enum Elf<'a> {
		Elf32 {
			bytes: &'a [u8],
			header: Header32,
			pheaders: Vec<ProgramHeader32>,
			sheaders: Vec<SectionHeader32>,
		},
		Elf64 {
			bytes: &'a [u8],
			header: Header64,
			pheaders: Vec<ProgramHeader64>,
			sheaders: Vec<SectionHeader64>,
		},
	}

	impl<'a> Elf<'a> {
		pub fn from_bytes(bytes: &'a [u8]) -> Result<Self> {
			let class = *core::ops::Index::index(bytes, EI_CLASS);

			match class {
				EI_CLASS_32 => Self::from_bytes_c32(bytes),
				EI_CLASS_64 => Self::from_bytes_c64(bytes),
				_ => return Err(Error::new(ErrorKind::InvalidClass)),
			}
		}

		fn from_bytes_c32(bytes: &'a [u8]) -> Result<Self> {
			let header = Header32::from_bytes(bytes)?;
			assert_eq!(header.e_ident.ei_class(), EI_CLASS_32);
			let endianness = header.e_ident.ei_data();

			// ProgramHeader
			let pheaders = {
				let ph_offset = header.e_phoff;
				let ph_count = header.e_phnum;
				let ph_size = header.e_phentsize;

				let mut pheaders = Vec::with_capacity(ph_count as usize);

				for idx in 0..ph_count {
					let start =
						(ph_offset + (idx as u32 * ph_size as u32)) as usize;
					let ph = ProgramHeader32::from_bytes(
						endianness,
						core::ops::Index::index(bytes, start..),
					)?;

					pheaders.push(ph);
				}

				pheaders
			};

			// SectionHeader
			let sheaders = {
				let sh_offset = header.e_shoff;
				let sh_count = header.e_shnum;
				let sh_size = header.e_shentsize;

				let mut sheaders = Vec::with_capacity(sh_count as usize);

				for idx in 0..sh_count {
					let start =
						(sh_offset + (idx as u32 * sh_size as u32)) as usize;
					let sh = SectionHeader32::from_bytes(
						endianness,
						core::ops::Index::index(bytes, start..),
					)?;

					sheaders.push(sh);
				}

				sheaders
			};

			Ok(Self::Elf32 { bytes, header, pheaders, sheaders })
		}

		fn from_bytes_c64(bytes: &'a [u8]) -> Result<Self> {
			#[cfg(not(target_pointer_width = "64"))]
			compile_error!("Needs 64 bits");

			let header = Header64::from_bytes(bytes)?;
			assert_eq!(header.e_ident.ei_class(), EI_CLASS_64);
			let endianness = header.e_ident.ei_data();

			// ProgramHeader
			let pheaders = {
				let ph_offset = header.e_phoff;
				let ph_count = header.e_phnum;
				let ph_size = header.e_phentsize;

				let mut pheaders = Vec::with_capacity(ph_count as usize);

				for idx in 0..ph_count {
					let start =
						(ph_offset + (idx as u64 * ph_size as u64)) as usize;
					let ph = ProgramHeader64::from_bytes(
						endianness,
						core::ops::Index::index(bytes, start..),
					)?;

					pheaders.push(ph);
				}

				pheaders
			};

			// SectionHeader
			let sheaders = {
				let sh_offset = header.e_shoff;
				let sh_count = header.e_shnum;
				let sh_size = header.e_shentsize;

				let mut sheaders = Vec::with_capacity(sh_count as usize);

				for idx in 0..sh_count {
					let start =
						(sh_offset + (idx as u64 * sh_size as u64)) as usize;
					let sh = SectionHeader64::from_bytes(
						endianness,
						core::ops::Index::index(bytes, start..),
					)?;

					sheaders.push(sh);
				}

				sheaders
			};

			Ok(Self::Elf64 { bytes, header, pheaders, sheaders })
		}
	}
}
