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
	/// # References
	/// - <https://refspecs.linuxbase.org/elf/elf.pdf>
	/// - <http://www.sco.com/developers/gabi/2000-07-17/ch4.eheader.html>
	/// - <https://en.wikipedia.org/wiki/Executable_and_Linkable_Format>
	pub mod consts {
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
						_ => "UNKNOWN",
					}
				}
			};
		}

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
				def_consts! {
					ei_class : u8 : ei_class_as_str => {
						/// Signifies 32-bit format.
						EI_CLASS_32 : "32-bit" = 1,

						/// Signifies 64-bit format.
						EI_CLASS_64 : "64-bit" = 2,
					}
				}
			}

			pub mod data {
				def_consts! {
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
				pub const EI_VERSION_CURRENT: u8 = 1;
			}

			pub mod osabi {
				def_consts! {
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
			def_consts! {
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
			def_consts! {
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
