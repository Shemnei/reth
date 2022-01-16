// Enable/disable `no_std` depending on the feature
#![cfg_attr(not(feature = "std"), no_std)]

pub mod error {
	use core::fmt;

	pub type Result<T, E = Error> = core::result::Result<T, E>;

	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
	pub enum ErrorKind {
		InsufficantSize,
		InvalidMagic,
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
	/// References:
	///     - <https://refspecs.linuxbase.org/elf/elf.pdf>
	///     - <http://www.sco.com/developers/gabi/2000-07-17/ch4.eheader.html>
	///     - <https://en.wikipedia.org/wiki/Executable_and_Linkable_Format>
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
				/// Field `ei_class`: Signifies 32-bit format.
				pub const EI_CLASS_32: u8 = 1;

				/// Field `ei_class`: Signifies 64-bit format.
				pub const EI_CLASS_64: u8 = 2;
			}

			pub mod data {
				/// Field `ei_data`: Signifies little endianness.
				pub const EI_DATA_LE: u8 = 1;

				/// Field `ei_data`: Signifies big endianness.
				pub const EI_DATA_BE: u8 = 2;
			}

			pub mod version {
				/// Field `ei_version`: Original and current version.
				pub const EI_VERSION_CURRENT: u8 = 1;
			}

			pub mod osabi {
				/// Field `ei_osabi`: System V.
				pub const EI_OSABI_SYSTEMV: u8 = 0x00;

				/// Field `ei_osabi`: HP-UX.
				pub const EI_OSABI_HPUX: u8 = 0x01;

				/// Field `ei_osabi`: NetBSD.
				pub const EI_OSABI_NETBSD: u8 = 0x02;

				/// Field `ei_osabi`: Linux.
				pub const EI_OSABI_LINUX: u8 = 0x03;

				/// Field `ei_osabi`: GNU Hurd.
				pub const EI_OSABI_GNUHURD: u8 = 0x04;

				/// Field `ei_osabi`: Solaris.
				pub const EI_OSABI_SOLARIS: u8 = 0x06;

				/// Field `ei_osabi`: AIX.
				pub const EI_OSABI_AIX: u8 = 0x07;

				/// Field `ei_osabi`: IRIX.
				pub const EI_OSABI_IRIX: u8 = 0x08;

				/// Field `ei_osabi`: FreeBSD.
				pub const EI_OSABI_FREEBSD: u8 = 0x09;

				/// Field `ei_osabi`: Tru64.
				pub const EI_OSABI_TRU64: u8 = 0x0a;

				/// Field `ei_osabi`: Novell Modesto.
				pub const EI_OSABI_NOVELLMODESTO: u8 = 0x0b;

				/// Field `ei_osabi`: OpenBSD.
				pub const EI_OSABI_OPENBSD: u8 = 0x0c;

				/// Field `ei_osabi`: OpenVMS.
				pub const EI_OSABI_OPENVMS: u8 = 0x0d;

				/// Field `ei_osabi`: NonStop Kernel.
				pub const EI_OSABI_NONSTOPKERNEL: u8 = 0x0e;

				/// Field `ei_osabi`: AROS.
				pub const EI_OSABI_AROS: u8 = 0x0f;

				/// Field `ei_osabi`: Fenix OS.
				pub const EI_OSABI_FENIXOS: u8 = 0x10;

				/// Field `ei_osabi`: CloudABI.
				pub const EI_OSABI_CLOUDABI: u8 = 0x11;

				/// Field `ei_osabi`: Stratus Technologies OpenVOS.
				pub const EI_OSABI_OPENVOS: u8 = 0x12;
			}
		}

		pub mod typ {
			/// Field `e_type`: ET_NONE.
			pub const E_TYPE_ET_NONE: u16 = 0x0000;

			/// Field `e_type`: ET_REL.
			pub const E_TYPE_ET_REL: u16 = 0x0001;

			/// Field `e_type`: ET_EXEC.
			pub const E_TYPE_ET_EXEC: u16 = 0x0002;

			/// Field `e_type`: ET_DYN.
			pub const E_TYPE_ET_DYN: u16 = 0x0003;

			/// Field `e_type`: ET_CORE.
			pub const E_TYPE_ET_CORE: u16 = 0x0004;

			/// Field `e_type`: ET_LOOS.
			pub const E_TYPE_ET_LOOS: u16 = 0xfe00;

			/// Field `e_type`: ET_HIOS.
			pub const E_TYPE_ET_HIOS: u16 = 0xfeff;

			/// Field `e_type`: ET_LOPROC.
			pub const E_TYPE_ET_LOPROC: u16 = 0xff00;

			/// Field `e_type`: ET_HIPROC.
			pub const E_TYPE_ET_HIPROC: u16 = 0xffff;
		}

		pub mod machine {
			/// Field `e_machine`: Unspecified.
			pub const E_MACHINE_UNSPECIFIED: u16 = 0x0000;

			/// Field `e_machine`: AT&T WE 32100.
			pub const E_MACHINE_ATTWE32100: u16 = 0x0001;

			/// Field `e_machine`: SPARC.
			pub const E_MACHINE_SPARC: u16 = 0x0002;

			/// Field `e_machine`: x86.
			pub const E_MACHINE_X86: u16 = 0x0003;

			/// Field `e_machine`: Motorola 68000 (M68k).
			pub const E_MACHINE_MOTOROLA68000: u16 = 0x0004;

			/// Field `e_machine`: Motorola 88000 (M88k).
			pub const E_MACHINE_MOTOROLA88000: u16 = 0x0005;

			/// Field `e_machine`: Intel MCU.
			pub const E_MACHINE_INTELMCU: u16 = 0x0006;

			/// Field `e_machine`: Intel 80860.
			pub const E_MACHINE_INTEL80860: u16 = 0x0007;

			/// Field `e_machine`: MIPS.
			pub const E_MACHINE_MIPS: u16 = 0x0008;

			/// Field `e_machine`: IBM System/370.
			pub const E_MACHINE_IBM370: u16 = 0x0009;

			/// Field `e_machine`: MIPS RS3000 Little-endian.
			pub const E_MACHINE_MIPSRS3000LE: u16 = 0x000a;

			/// Field `e_machine`: Hewlett-Packard PA-RISC.
			pub const E_MACHINE_HPPARISC: u16 = 0x000e;

			/// Field `e_machine`: Intel 80960.
			pub const E_MACHINE_INTEL80960: u16 = 0x0013;

			/// Field `e_machine`: PowerPC.
			pub const E_MACHINE_POWERPC: u16 = 0x0014;

			/// Field `e_machine`: PowerPC (64-bit).
			pub const E_MACHINE_POWERPC64: u16 = 0x0015;

			/// Field `e_machine`: S390, including S390x.
			pub const E_MACHINE_S390: u16 = 0x0016;

			/// Field `e_machine`: IBM SPU/SPC.
			pub const E_MACHINE_IBMSPUSPC: u16 = 0x0017;

			/// Field `e_machine`: NEC V800.
			pub const E_MACHINE_NECV800: u16 = 0x0024;

			/// Field `e_machine`: Fujitsu FR20.
			pub const E_MACHINE_FUJITSUFR20: u16 = 0x0025;

			/// Field `e_machine`: TRW RH-32.
			pub const E_MACHINE_TRWRH32: u16 = 0x0026;

			/// Field `e_machine`: Motorola RCE.
			pub const E_MACHINE_MOTOROLARCE: u16 = 0x0027;

			/// Field `e_machine`: ARM (up to ARMv7/Aarch32).
			pub const E_MACHINE_ARM: u16 = 0x0028;

			/// Field `e_machine`: Digital Alpha.
			pub const E_MACHINE_DIGITALALPHA: u16 = 0x0029;

			/// Field `e_machine`: SuperH.
			pub const E_MACHINE_SUPERH: u16 = 0x002a;

			/// Field `e_machine`: SPARC Version 9.
			pub const E_MACHINE_SPARC9: u16 = 0x002b;

			/// Field `e_machine`: Siemens TriCore embedded processor.
			pub const E_MACHINE_SIEMENSTRICORE: u16 = 0x002c;

			/// Field `e_machine`: Argonaut RISC Core.
			pub const E_MACHINE_ARGONAUTRISCCORE: u16 = 0x002d;

			/// Field `e_machine`: Hitachi H8/300.
			pub const E_MACHINE_HITACHIH8300: u16 = 0x002e;

			/// Field `e_machine`: Hitachi H8/300H.
			pub const E_MACHINE_HITACHIH8300H: u16 = 0x002f;

			/// Field `e_machine`: Hitachi H8S.
			pub const E_MACHINE_HITACHIH8S: u16 = 0x0030;

			/// Field `e_machine`: Hitachi H8/500.
			pub const E_MACHINE_HITACHIH8500: u16 = 0x0031;

			/// Field `e_machine`: IA-64.
			pub const E_MACHINE_IA64: u16 = 0x0032;

			/// Field `e_machine`: Stanford MIPS-X.
			pub const E_MACHINE_STANFORDMIPSX: u16 = 0x0033;

			/// Field `e_machine`: Motorola ColdFire.
			pub const E_MACHINE_MOTOROLACOLDFIRE: u16 = 0x0034;

			/// Field `e_machine`: Motorola M68HC12.
			pub const E_MACHINE_MOTOROLAM68HC12: u16 = 0x0035;

			/// Field `e_machine`: Fujitsu MMA Multimedia Accelerator.
			pub const E_MACHINE_FUJITSUMMA: u16 = 0x0036;

			/// Field `e_machine`: Siemens PCP.
			pub const E_MACHINE_SIEMENSPCP: u16 = 0x0037;

			/// Field `e_machine`: Sony nCPU embedded RISC processor.
			pub const E_MACHINE_SONYNCPURISC: u16 = 0x0038;

			/// Field `e_machine`: Denso NDR1 microprocessor.
			pub const E_MACHINE_DENSONDR1: u16 = 0x0039;

			/// Field `e_machine`: Motorola Star*Core processor.
			pub const E_MACHINE_MOTOROLASTARCORE: u16 = 0x003a;

			/// Field `e_machine`: Toyota ME16 processor.
			pub const E_MACHINE_TOYOTAME16: u16 = 0x003b;

			/// Field `e_machine`: STMicroelectronics ST100 processor.
			pub const E_MACHINE_STMST100: u16 = 0x003c;

			/// Field `e_machine`: Advanced Logic Corp. Tinyj embedded processor family.
			pub const E_MACHINE_ALCTINYJ: u16 = 0x003d;

			/// Field `e_machine`: AMD x86-64.
			pub const E_MACHINE_AMD8664: u16 = 0x003e;

			/// Field `e_machine`: TMS320C6000 Family.
			pub const E_MACHINE_TMS320C6000: u16 = 0x008c;

			/// Field `e_machine`: MCST Elbrus e2k.
			pub const E_MACHINE_MCSTELBRUSE2K: u16 = 0x00af;

			/// Field `e_machine`: ARM 64-bits (ARMv8/Aarch64).
			pub const E_MACHINE_ARM64: u16 = 0x00b7;

			/// Field `e_machine`: RISC-V.
			pub const E_MACHINE_RISCV: u16 = 0x00f3;

			/// Field `e_machine`: Berkeley Packet Filter.
			pub const E_MACHINE_BPF: u16 = 0x00f7;

			/// Field `e_machine`: WDC 65C816.
			pub const E_MACHINE_WDC65C816: u16 = 0x0101;
		}
	}

	mod stringify {
		#[rustfmt::skip]
		pub fn ei_class_as_str(ei_class: u8) -> &'static str {
			match ei_class {
				crate::header::consts::ident::class::EI_CLASS_32 => "32-bit",
				crate::header::consts::ident::class::EI_CLASS_64 => "64-bit",
				_ => "UNKNOWN",
			}
		}

		#[rustfmt::skip]
		pub fn ei_data_as_str(ei_data: u8) -> &'static str {
			match ei_data {
				crate::header::consts::ident::data::EI_DATA_LE => "little-endian",
				crate::header::consts::ident::data::EI_DATA_BE => "big-endian",
				_ => "UNKNOWN",
			}
		}

		#[rustfmt::skip]
		pub fn ei_osabi_as_str(ei_osabi: u8) -> &'static str {
			match ei_osabi {
				crate::header::consts::ident::osabi::EI_OSABI_SYSTEMV => "System V",
				crate::header::consts::ident::osabi::EI_OSABI_HPUX => "HP-UX",
				crate::header::consts::ident::osabi::EI_OSABI_NETBSD => "NetBSD",
				crate::header::consts::ident::osabi::EI_OSABI_LINUX => "Linux",
				crate::header::consts::ident::osabi::EI_OSABI_GNUHURD => "GNU Hurd",
				crate::header::consts::ident::osabi::EI_OSABI_SOLARIS => "Solaris",
				crate::header::consts::ident::osabi::EI_OSABI_AIX => "AIX",
				crate::header::consts::ident::osabi::EI_OSABI_IRIX => "IRIX",
				crate::header::consts::ident::osabi::EI_OSABI_FREEBSD => "FreeBSD",
				crate::header::consts::ident::osabi::EI_OSABI_TRU64 => "Tru64",
				crate::header::consts::ident::osabi::EI_OSABI_NOVELLMODESTO => "Novell Modesto",
				crate::header::consts::ident::osabi::EI_OSABI_OPENBSD => "OpenBSD",
				crate::header::consts::ident::osabi::EI_OSABI_OPENVMS => "OpenVMS",
				crate::header::consts::ident::osabi::EI_OSABI_NONSTOPKERNEL => "NonStop Kernel",
				crate::header::consts::ident::osabi::EI_OSABI_AROS => "AROS",
				crate::header::consts::ident::osabi::EI_OSABI_FENIXOS => "Fenix OS",
				crate::header::consts::ident::osabi::EI_OSABI_CLOUDABI => "CloudABI",
				crate::header::consts::ident::osabi::EI_OSABI_OPENVOS => "Stratus Technologies OpenVOS",
				_ => "UNKNOWN",
			}
		}

		#[rustfmt::skip]
		pub fn e_type_as_str(e_type: u16) -> &'static str {
			match e_type {
				crate::header::consts::typ::E_TYPE_ET_NONE => "ET_NONE",
				crate::header::consts::typ::E_TYPE_ET_REL => "ET_REL",
				crate::header::consts::typ::E_TYPE_ET_EXEC => "ET_EXEC",
				crate::header::consts::typ::E_TYPE_ET_DYN => "ET_DYN",
				crate::header::consts::typ::E_TYPE_ET_CORE => "ET_CORE",
				crate::header::consts::typ::E_TYPE_ET_LOOS => "ET_LOOS",
				crate::header::consts::typ::E_TYPE_ET_HIOS => "ET_HIOS",
				crate::header::consts::typ::E_TYPE_ET_LOPROC => "ET_LOPROC",
				crate::header::consts::typ::E_TYPE_ET_HIPROC => "ET_HIPROC",
				_ => "UNKNOWN",
			}
		}

		// TODO: replace with macro?
		#[rustfmt::skip]
		pub fn e_machine_as_str(e_machine: u16) -> &'static str {
			match e_machine {
				crate::header::consts::machine::E_MACHINE_UNSPECIFIED => "Unspecified",
				crate::header::consts::machine::E_MACHINE_ATTWE32100 => "AT&T WE 32100",
				crate::header::consts::machine::E_MACHINE_SPARC => "SPARC",
				crate::header::consts::machine::E_MACHINE_X86 => "x86",
				crate::header::consts::machine::E_MACHINE_MOTOROLA68000 => "Motorola 68000 (M68k)",
				crate::header::consts::machine::E_MACHINE_MOTOROLA88000 => "Motorola 88000 (M88k)",
				crate::header::consts::machine::E_MACHINE_INTELMCU => "Intel MCU",
				crate::header::consts::machine::E_MACHINE_INTEL80860 => "Intel 80860",
				crate::header::consts::machine::E_MACHINE_MIPS => "MIPS",
				crate::header::consts::machine::E_MACHINE_IBM370 => "IBM System/370",
				crate::header::consts::machine::E_MACHINE_MIPSRS3000LE => "MIPS RS3000 Little-endian",
				crate::header::consts::machine::E_MACHINE_HPPARISC => "Hewlett-Packard PA-RISC",
				crate::header::consts::machine::E_MACHINE_INTEL80960 => "Intel 80960",
				crate::header::consts::machine::E_MACHINE_POWERPC => "PowerPC",
				crate::header::consts::machine::E_MACHINE_POWERPC64 => "PowerPC (64-bit)",
				crate::header::consts::machine::E_MACHINE_S390 => "S390, including S390x",
				crate::header::consts::machine::E_MACHINE_IBMSPUSPC => "IBM SPU/SPC",
				crate::header::consts::machine::E_MACHINE_NECV800 => "NEC V800",
				crate::header::consts::machine::E_MACHINE_FUJITSUFR20 => "Fujitsu FR20",
				crate::header::consts::machine::E_MACHINE_TRWRH32 => "TRW RH-32",
				crate::header::consts::machine::E_MACHINE_MOTOROLARCE => "Motorola RCE",
				crate::header::consts::machine::E_MACHINE_ARM => "ARM (up to ARMv7/Aarch32)",
				crate::header::consts::machine::E_MACHINE_DIGITALALPHA => "Digital Alpha",
				crate::header::consts::machine::E_MACHINE_SUPERH => "SuperH",
				crate::header::consts::machine::E_MACHINE_SPARC9 => "SPARC Version 9",
				crate::header::consts::machine::E_MACHINE_SIEMENSTRICORE => "Siemens TriCore embedded processor",
				crate::header::consts::machine::E_MACHINE_ARGONAUTRISCCORE => "Argonaut RISC Core",
				crate::header::consts::machine::E_MACHINE_HITACHIH8300 => "Hitachi H8/300",
				crate::header::consts::machine::E_MACHINE_HITACHIH8300H => "Hitachi H8/300H",
				crate::header::consts::machine::E_MACHINE_HITACHIH8S => "Hitachi H8S",
				crate::header::consts::machine::E_MACHINE_HITACHIH8500 => "Hitachi H8/500",
				crate::header::consts::machine::E_MACHINE_IA64 => "IA-64",
				crate::header::consts::machine::E_MACHINE_STANFORDMIPSX => "Stanford MIPS-X",
				crate::header::consts::machine::E_MACHINE_MOTOROLACOLDFIRE => "Motorola ColdFire",
				crate::header::consts::machine::E_MACHINE_MOTOROLAM68HC12 => "Motorola M68HC12",
				crate::header::consts::machine::E_MACHINE_FUJITSUMMA => "Fujitsu MMA Multimedia Accelerator",
				crate::header::consts::machine::E_MACHINE_SIEMENSPCP => "Siemens PCP",
				crate::header::consts::machine::E_MACHINE_SONYNCPURISC => "Sony nCPU embedded RISC processor",
				crate::header::consts::machine::E_MACHINE_DENSONDR1 => "Denso NDR1 microprocessor",
				crate::header::consts::machine::E_MACHINE_MOTOROLASTARCORE => "Motorola Star*Core processor",
				crate::header::consts::machine::E_MACHINE_TOYOTAME16 => "Toyota ME16 processor",
				crate::header::consts::machine::E_MACHINE_STMST100 => "STMicroelectronics ST100 processor",
				crate::header::consts::machine::E_MACHINE_ALCTINYJ => "Advanced Logic Corp. Tinyj embedded processor family",
				crate::header::consts::machine::E_MACHINE_AMD8664 => "AMD x86-64",
				crate::header::consts::machine::E_MACHINE_TMS320C6000 => "TMS320C6000 Family",
				crate::header::consts::machine::E_MACHINE_MCSTELBRUSE2K => "MCST Elbrus e2k",
				crate::header::consts::machine::E_MACHINE_ARM64 => "ARM 64-bits (ARMv8/Aarch64)",
				crate::header::consts::machine::E_MACHINE_RISCV => "RISC-V",
				crate::header::consts::machine::E_MACHINE_BPF => "Berkeley Packet Filter",
				crate::header::consts::machine::E_MACHINE_WDC65C816 => "WDC 65C816",
				_ => "UNKNOWN",
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
				crate::header::stringify::ei_class_as_str(self.ei_class()),
				crate::header::stringify::ei_data_as_str(self.ei_data()),
				self.ei_version(),
				crate::header::stringify::ei_osabi_as_str(self.ei_osabi()),
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

				/// Field `e_machine`: Specifies a target instruction set architecture.
				pub e_machine: u16,

				/// Field `e_version`: Specifies the version of elf (`1` for the original version).
				pub e_version: u32,

				/// Field `e_entry`: Memory address of the entry point.
				pub e_entry: $size,

				/// Field `e_phoff`: Points to the start of the program header table.
				pub e_phoff: $size,

				/// Field `e_shoff`: Points to the start of the section header table.
				pub e_shoff: $size,

				/// Field `e_flags`: Flags (interpretation of the field depends on the target
				/// architecture).
				pub e_flags: u32,

				/// Field `e_ehsize`: Contains the size of this [`Header`].
				pub e_ehsize: u16,

				/// Field `e_phentsize`: Contains the size of a program header table entry.
				pub e_phentsize: u16,

				/// Field `e_phnum`: Contains the number of entries in the program header table.
				pub e_phnum: u16,

				/// Field `e_shentsize`: Contains the size of a section header table entry.
				pub e_shentsize: u16,

				/// Field `e_shnum`: Contains the number of entries in the section header table.
				pub e_shnum: u16,

				/// Field `e_shstrndx`: Contains index of the section header table entry that
				/// contains the section names.
				pub e_shstrndx: u16,
			}

			impl Header {
                #[allow(unused_assignments, clippy::eval_order_dependence)]
				pub fn from_bytes(mut bytes: &[u8]) -> crate::error::Result<Self> {
                    /// Consumes bytes from the `bytes` slice and converts them
                    /// to the requested type. The `bytes` slice will be advanced
                    /// by the amount of bytes consumed for the type.
					macro_rules! consume {
                        // Consumes a single byte from `bytes`.
                        ( u8 ) => {{
                            consume!(@single)
                        }};
                        // Consumes `size_of<$type>` bytes and converts them to
                        // `$type` with big endianness.
						( be $type:ty ) => {{
							const SIZE: usize = core::mem::size_of::<$type>();

							<$type>::f(consume!(@arr SIZE))
						}};
                        // Consumes `size_of<$type>` bytes and converts them to
                        // `$type` with little endianness.
						( le $type:ty ) => {{
							const SIZE: usize = core::mem::size_of::<$type>();

							<$type>::from_le_bytes(consume!(@arr SIZE))
						}};
                        // Consumes `size_of<$type>` bytes and converts them to
                        // the requested endianness (@see `EI_DATA_LE` and
                        // `EI_DATA_BE` for possible values).
						( $endianness:expr => $type:ty ) => {{
							const SIZE: usize = core::mem::size_of::<$type>();

                            // TODO: return err
                            match $endianness {
                                crate::header::consts::ident::data::EI_DATA_BE => Ok(<$type>::from_be_bytes(consume!(@arr SIZE))),
                                crate::header::consts::ident::data::EI_DATA_LE => Ok(<$type>::from_le_bytes(consume!(@arr SIZE))),
                                _ => Err(crate::error::Error::new(crate::error::ErrorKind::UnknownEndianess))
							}
						}};
                        // Consumes `$len` bytes and returns them as array.
						(  $len:expr ) => {{
                            consume!(@arr $len)
                        }};
                        // PRIVATE: Shared code to consume a single byte.
                        ( @single ) => {{
                            let buf = bytes[0];
                            bytes = &bytes[1..];
                            buf
                        }};
                        // PRIVATE: Shared code to consume a byte array.
						( @arr $len:expr ) => {{
							let mut buf = [0u8; $len];
							let (left, right) = bytes.split_at($len);
							buf.copy_from_slice(left);
							bytes = right;
							buf
						}};
					}

					if bytes.len() < core::mem::size_of::<Self>() {
                        return Err(crate::error::Error::new(crate::error::ErrorKind::InsufficantSize))
					}


                    const SIZE_IDENT: usize = core::mem::size_of::<crate::header::Ident>();

                    let e_ident = crate::header::Ident(consume!(SIZE_IDENT));
                    if e_ident.ei_mag() != &[0x7f, 0x45, 0x4c, 0x46] {
                        return Err(crate::error::Error::new(crate::error::ErrorKind::InvalidMagic))
                    }
                    let endianness = e_ident.ei_data();

                    Ok(Self {
                        e_ident,
                        e_type: consume!(endianness => u16)?,
                        e_machine: consume!(endianness => u16)?,
                        e_version: consume!(endianness => u32)?,
                        e_entry: consume!(endianness => $size)?,
                        e_phoff: consume!(endianness => $size)?,
                        e_shoff: consume!(endianness => $size)?,
                        e_flags: consume!(endianness => u32)?,
                        e_ehsize: consume!(endianness => u16)?,
                        e_phentsize: consume!(endianness => u16)?,
                        e_phnum: consume!(endianness => u16)?,
                        e_shentsize: consume!(endianness => u16)?,
                        e_shnum: consume!(endianness => u16)?,
                        e_shstrndx: consume!(endianness => u16)?,
                    })
				}
			}

			// TODO: impl display
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
	e_entry    : {:#x}
	e_phoff    : {}
	e_shoff    : {}
	e_flags    : {:#b}
	e_ehsize   : {}
	e_phentsize: {}
	e_phnum    : {}
	e_shentsize: {}
	e_shnum    : {}
	e_shstrndx : {}"#,
						crate::header::stringify::ei_class_as_str(self.e_ident.ei_class()),
						crate::header::stringify::ei_data_as_str(self.e_ident.ei_data()),
						self.e_ident.ei_version(),
						crate::header::stringify::ei_osabi_as_str(self.e_ident.ei_osabi()),
						self.e_ident.ei_abiversion(),
						crate::header::stringify::e_type_as_str(self.e_type),
						crate::header::stringify::e_machine_as_str(self.e_machine),
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
						self.e_shstrndx
					))

				}
			}
		};
	}

	pub mod u32 {
		header!(u32);
	}

	pub mod u64 {
		header!(u64);

		#[cfg(test)]
		mod tests {
			use super::*;

			#[test]
			#[cfg(feature = "std")]
			fn punktf_header() {
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
