//! POSIX getopt + GNU getopt_long for SaltyOS
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Implements POSIX.1-2008 `getopt()` and GNU `getopt_long()`/`getopt_long_only()`.
//! Based on the POSIX specification with GNU extensions.

// -----------------------------------------------------------------------
// Exported globals
// -----------------------------------------------------------------------

/// Pointer to the argument value for options that take arguments.
#[unsafe(no_mangle)]
pub static mut optarg: *mut u8 = core::ptr::null_mut();

/// Index of the next element of argv to be processed.
#[unsafe(no_mangle)]
pub static mut optind: i32 = 1;

/// If non-zero, print error messages to stderr.
#[unsafe(no_mangle)]
pub static mut opterr: i32 = 1;

/// The option character that caused the error.
#[unsafe(no_mangle)]
pub static mut optopt: i32 = b'?' as i32;

/// When set to non-zero, reset all internal parsing state.
#[unsafe(no_mangle)]
pub static mut optreset: i32 = 0;

// -----------------------------------------------------------------------
// Internal state
// -----------------------------------------------------------------------

/// Position within the current argv element being parsed.
static mut NEXTCHAR: *const u8 = core::ptr::null();

/// Stop at first non-option argument.
static mut POSIXLY_CORRECT: i32 = 0;

/// Return non-option arguments as '\1'.
static mut RETURN_NONOPTS: i32 = 0;

// -----------------------------------------------------------------------
// Long option struct + constants
// -----------------------------------------------------------------------

/// GNU `struct option` for `getopt_long()`.
#[repr(C)]
pub struct option {
    pub name: *const u8,
    pub has_arg: i32,
    pub flag: *mut i32,
    pub val: i32,
}

pub const NO_ARGUMENT: i32 = 0;
pub const REQUIRED_ARGUMENT: i32 = 1;
pub const OPTIONAL_ARGUMENT: i32 = 2;

#[unsafe(no_mangle)]
pub static no_argument: i32 = NO_ARGUMENT;
#[unsafe(no_mangle)]
pub static required_argument: i32 = REQUIRED_ARGUMENT;
#[unsafe(no_mangle)]
pub static optional_argument: i32 = OPTIONAL_ARGUMENT;

// -----------------------------------------------------------------------
// Helpers for error output
// -----------------------------------------------------------------------

/// Write the program name (argv[0]) to stderr.
unsafe fn write_progname(argv: *const *mut u8) {
    unsafe {
        let se = *(&raw const crate::stdio::stderr);
        let arg0 = *argv;
        if !arg0.is_null() {
            crate::stdio::fputs(arg0, se);
        }
    }
}

/// Write a string literal (null-terminated) to stderr.
unsafe fn write_str(s: &[u8]) {
    unsafe {
        let se = *(&raw const crate::stdio::stderr);
        crate::stdio::fputs(s.as_ptr(), se);
    }
}

/// Write a single byte as a character to stderr.
unsafe fn write_char(c: u8) {
    unsafe {
        let se = *(&raw const crate::stdio::stderr);
        crate::stdio::fputc(c as i32, se);
    }
}

// Error message helpers matching the C original's fprintf patterns.

/// `progname: invalid option -- 'c'\n`
unsafe fn err_invalid_option(argv: *const *mut u8, c: u8) {
    unsafe {
        write_progname(argv);
        write_str(b": invalid option -- '\0");
        write_char(c);
        write_str(b"'\n\0");
    }
}

/// `progname: option requires an argument -- 'c'\n`
unsafe fn err_requires_argument(argv: *const *mut u8, c: u8) {
    unsafe {
        write_progname(argv);
        write_str(b": option requires an argument -- '\0");
        write_char(c);
        write_str(b"'\n\0");
    }
}

/// `progname: option 'str' is ambiguous\n`
unsafe fn err_ambiguous(argv: *const *mut u8, optstr: *const u8) {
    unsafe {
        write_progname(argv);
        write_str(b": option '\0");
        let se = *(&raw const crate::stdio::stderr);
        crate::stdio::fputs(optstr, se);
        write_str(b"' is ambiguous\n\0");
    }
}

/// `progname: option '--name' doesn't allow an argument\n`
unsafe fn err_no_arg_allowed(argv: *const *mut u8, name: *const u8) {
    unsafe {
        write_progname(argv);
        write_str(b": option '--\0");
        let se = *(&raw const crate::stdio::stderr);
        crate::stdio::fputs(name, se);
        write_str(b"' doesn't allow an argument\n\0");
    }
}

/// `progname: option '--name' requires an argument\n`
unsafe fn err_long_requires_argument(argv: *const *mut u8, name: *const u8) {
    unsafe {
        write_progname(argv);
        write_str(b": option '--\0");
        let se = *(&raw const crate::stdio::stderr);
        crate::stdio::fputs(name, se);
        write_str(b"' requires an argument\n\0");
    }
}

/// `progname: unrecognized option 'str'\n`
unsafe fn err_unrecognized(argv: *const *mut u8, optstr: *const u8) {
    unsafe {
        write_progname(argv);
        write_str(b": unrecognized option '\0");
        let se = *(&raw const crate::stdio::stderr);
        crate::stdio::fputs(optstr, se);
        write_str(b"'\n\0");
    }
}

// -----------------------------------------------------------------------
// Internal: parse optstring prefix characters
// -----------------------------------------------------------------------

/// Parse optstring prefix characters:
///   '+' -> POSIXLY_CORRECT (stop at first non-option)
///   '-' -> return non-option args as '\1'
///   ':' -> return ':' instead of '?' for missing arg
///
/// Returns the optstring pointer advanced past prefix chars, and sets
/// `*colon_mode` accordingly.
unsafe fn parse_prefix(mut optstring: *const u8, colon_mode: &mut i32) -> *const u8 {
    unsafe {
        *(&raw mut POSIXLY_CORRECT) = 0;
        *(&raw mut RETURN_NONOPTS) = 0;
        *colon_mode = 0;

        loop {
            let ch = *optstring;
            if ch == b'+' {
                *(&raw mut POSIXLY_CORRECT) = 1;
                optstring = optstring.add(1);
            } else if ch == b'-' {
                *(&raw mut RETURN_NONOPTS) = 1;
                optstring = optstring.add(1);
            } else if ch == b':' {
                *colon_mode = 1;
                optstring = optstring.add(1);
            } else {
                break;
            }
        }
        optstring
    }
}

// -----------------------------------------------------------------------
// Internal: match a long option name
// -----------------------------------------------------------------------

/// Match a long option name against the longopts array.
/// Returns index into longopts, or -1 if not found.
/// Sets `*exact` to 1 if an exact match, 0 otherwise.
/// Sets `*ambig` to 1 if the prefix is ambiguous.
unsafe fn match_long(
    arg: *const u8,
    longopts: *const option,
    exact: &mut i32,
    ambig: &mut i32,
) -> i32 {
    unsafe {
        *exact = 0;
        *ambig = 0;

        let eq = crate::string::strchr(arg, b'=' as i32);
        let arglen = if !eq.is_null() {
            (eq as usize) - (arg as usize)
        } else {
            crate::string::strlen(arg)
        };

        let mut matched: i32 = -1;
        let mut nmatches: i32 = 0;
        let mut i: i32 = 0;

        while !(*(longopts.offset(i as isize))).name.is_null() {
            let entry = &*(longopts.offset(i as isize));
            if crate::string::strncmp(arg, entry.name, arglen) == 0 {
                if crate::string::strlen(entry.name) == arglen {
                    // Exact match
                    *exact = 1;
                    return i;
                }
                // Prefix match
                matched = i;
                nmatches += 1;
            }
            i += 1;
        }

        if nmatches == 1 {
            return matched;
        }
        if nmatches > 1 {
            *ambig = 1;
        }
        -1
    }
}

// -----------------------------------------------------------------------
// getopt (short options only)
// -----------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getopt(argc: i32, argv: *const *mut u8, optstring: *const u8) -> i32 {
    unsafe {
        // Handle optreset
        if *(&raw const optreset) != 0 {
            *(&raw mut optreset) = 0;
            *(&raw mut optind) = 1;
            *(&raw mut NEXTCHAR) = core::ptr::null();
        }

        if *(&raw const optind) <= 0 {
            *(&raw mut optind) = 1;
            *(&raw mut NEXTCHAR) = core::ptr::null();
        }

        *(&raw mut optarg) = core::ptr::null_mut();

        let mut colon_mode: i32 = 0;
        let opts = parse_prefix(optstring, &mut colon_mode);

        let nextchar_val = *(&raw const NEXTCHAR);

        // If no more chars in current arg, advance to next
        if nextchar_val.is_null() || *nextchar_val == 0 {
            let cur_optind = *(&raw const optind);

            if cur_optind >= argc {
                return -1;
            }

            let cur_arg = *argv.offset(cur_optind as isize);

            // Check for "--" end-of-options marker
            if *cur_arg == b'-' && *cur_arg.add(1) == b'-' && *cur_arg.add(2) == 0 {
                *(&raw mut optind) += 1;
                return -1;
            }

            // Not an option?
            if *cur_arg != b'-' || *cur_arg.add(1) == 0 {
                if *(&raw const POSIXLY_CORRECT) != 0 {
                    return -1;
                }
                if *(&raw const RETURN_NONOPTS) != 0 {
                    *(&raw mut optarg) = *argv.offset(cur_optind as isize);
                    *(&raw mut optind) += 1;
                    return 1; // '\1' -- non-option argument
                }
                // Default: stop
                return -1;
            }

            *(&raw mut NEXTCHAR) = cur_arg.add(1);
        }

        // Current option character
        let nc = *(&raw const NEXTCHAR);
        let c = *nc;
        *(&raw mut NEXTCHAR) = nc.add(1);
        *(&raw mut optopt) = c as i32;

        // Look up in optstring
        let p = crate::string::strchr(opts, c as i32);
        if p.is_null() || c == b':' {
            if *(&raw const opterr) != 0 && colon_mode == 0 {
                err_invalid_option(argv, c);
            }
            let nc2 = *(&raw const NEXTCHAR);
            if *nc2 == 0 {
                *(&raw mut optind) += 1;
                *(&raw mut NEXTCHAR) = core::ptr::null();
            }
            return b'?' as i32;
        }

        // Does this option take an argument?
        if *p.add(1) == b':' {
            if *p.add(2) == b':' {
                // Optional argument (::) -- must be in same argv element
                let nc2 = *(&raw const NEXTCHAR);
                if *nc2 != 0 {
                    *(&raw mut optarg) = nc2 as *mut u8;
                    *(&raw mut NEXTCHAR) = core::ptr::null();
                } else {
                    *(&raw mut optarg) = core::ptr::null_mut();
                }
                *(&raw mut optind) += 1;
                *(&raw mut NEXTCHAR) = core::ptr::null();
            } else {
                // Required argument
                let nc2 = *(&raw const NEXTCHAR);
                if *nc2 != 0 {
                    *(&raw mut optarg) = nc2 as *mut u8;
                    *(&raw mut NEXTCHAR) = core::ptr::null();
                    *(&raw mut optind) += 1;
                } else if *(&raw const optind) + 1 < argc {
                    *(&raw mut optind) += 1;
                    *(&raw mut optarg) = *argv.offset(*(&raw const optind) as isize);
                    *(&raw mut optind) += 1;
                } else {
                    // Missing argument
                    *(&raw mut optind) += 1;
                    *(&raw mut NEXTCHAR) = core::ptr::null();
                    if colon_mode != 0 {
                        return b':' as i32;
                    }
                    if *(&raw const opterr) != 0 {
                        err_requires_argument(argv, c);
                    }
                    return b'?' as i32;
                }
            }
        } else {
            // No argument
            let nc2 = *(&raw const NEXTCHAR);
            if *nc2 == 0 {
                *(&raw mut optind) += 1;
                *(&raw mut NEXTCHAR) = core::ptr::null();
            }
        }

        c as i32
    }
}

// -----------------------------------------------------------------------
// getopt_long_internal (shared implementation)
// -----------------------------------------------------------------------

unsafe fn getopt_long_internal(
    argc: i32,
    argv: *const *mut u8,
    optstring: *const u8,
    longopts: *const option,
    longindex: *mut i32,
    long_only: i32,
) -> i32 {
    unsafe {
        // Handle optreset
        if *(&raw const optreset) != 0 {
            *(&raw mut optreset) = 0;
            *(&raw mut optind) = 1;
            *(&raw mut NEXTCHAR) = core::ptr::null();
        }

        if *(&raw const optind) <= 0 {
            *(&raw mut optind) = 1;
            *(&raw mut NEXTCHAR) = core::ptr::null();
        }

        *(&raw mut optarg) = core::ptr::null_mut();

        let mut colon_mode: i32 = 0;
        let opts = parse_prefix(optstring, &mut colon_mode);

        let nextchar_val = *(&raw const NEXTCHAR);

        // If no more chars in current arg, advance
        if nextchar_val.is_null() || *nextchar_val == 0 {
            let cur_optind = *(&raw const optind);

            if cur_optind >= argc {
                return -1;
            }

            let cur_arg = *argv.offset(cur_optind as isize);

            // "--" end marker
            if *cur_arg == b'-' && *cur_arg.add(1) == b'-' && *cur_arg.add(2) == 0 {
                *(&raw mut optind) += 1;
                return -1;
            }

            // Not an option?
            if *cur_arg != b'-' || *cur_arg.add(1) == 0 {
                if *(&raw const POSIXLY_CORRECT) != 0 {
                    return -1;
                }
                if *(&raw const RETURN_NONOPTS) != 0 {
                    *(&raw mut optarg) = *argv.offset(cur_optind as isize);
                    *(&raw mut optind) += 1;
                    return 1;
                }
                return -1;
            }

            *(&raw mut NEXTCHAR) = core::ptr::null();

            // Try long option first: "--foo" or (long_only) "-foo"
            if !longopts.is_null() {
                let is_long = *cur_arg == b'-' && *cur_arg.add(1) == b'-';
                let try_long =
                    is_long || (long_only != 0 && *cur_arg == b'-' && *cur_arg.add(2) != 0);

                if try_long {
                    let longarg = if is_long {
                        cur_arg.add(2)
                    } else {
                        cur_arg.add(1)
                    };

                    let mut exact_match: i32 = 0;
                    let mut ambig_match: i32 = 0;
                    let matched = match_long(longarg, longopts, &mut exact_match, &mut ambig_match);

                    if ambig_match != 0 {
                        if *(&raw const opterr) != 0 {
                            err_ambiguous(argv, cur_arg);
                        }
                        *(&raw mut optind) += 1;
                        return b'?' as i32;
                    }

                    if matched >= 0 {
                        let o = &*(longopts.offset(matched as isize));
                        let eq = crate::string::strchr(longarg, b'=' as i32);

                        if !longindex.is_null() {
                            *longindex = matched;
                        }

                        if o.has_arg == NO_ARGUMENT {
                            if !eq.is_null() {
                                if *(&raw const opterr) != 0 {
                                    err_no_arg_allowed(argv, o.name);
                                }
                                *(&raw mut optind) += 1;
                                return b'?' as i32;
                            }
                            *(&raw mut optind) += 1;
                        } else if o.has_arg == REQUIRED_ARGUMENT {
                            if !eq.is_null() {
                                // SAFETY: eq points to '=' within the arg;
                                // eq+1 is the start of the value string.
                                *(&raw mut optarg) = (eq as *mut u8).add(1);
                                *(&raw mut optind) += 1;
                            } else if *(&raw const optind) + 1 < argc {
                                *(&raw mut optind) += 1;
                                *(&raw mut optarg) = *argv.offset(*(&raw const optind) as isize);
                                *(&raw mut optind) += 1;
                            } else {
                                if *(&raw const opterr) != 0 {
                                    err_long_requires_argument(argv, o.name);
                                }
                                *(&raw mut optind) += 1;
                                return if colon_mode != 0 {
                                    b':' as i32
                                } else {
                                    b'?' as i32
                                };
                            }
                        } else {
                            // optional_argument
                            if !eq.is_null() {
                                // SAFETY: eq points to '=' within the arg;
                                // eq+1 is the start of the value string.
                                *(&raw mut optarg) = (eq as *mut u8).add(1);
                            }
                            *(&raw mut optind) += 1;
                        }

                        if !o.flag.is_null() {
                            *o.flag = o.val;
                            return 0;
                        }
                        return o.val;
                    }

                    // No long match for "--xxx": error
                    if is_long {
                        if *(&raw const opterr) != 0 {
                            err_unrecognized(argv, cur_arg);
                        }
                        *(&raw mut optind) += 1;
                        return b'?' as i32;
                    }
                    // long_only with no match: fall through to short opt
                }
            }

            // Set up for short option processing
            *(&raw mut NEXTCHAR) = (*argv.offset(cur_optind as isize)).add(1);
        }

        // Short option processing
        let nc = *(&raw const NEXTCHAR);
        let c = *nc;
        *(&raw mut NEXTCHAR) = nc.add(1);
        *(&raw mut optopt) = c as i32;

        let p = crate::string::strchr(opts, c as i32);
        if p.is_null() || c == b':' {
            if *(&raw const opterr) != 0 && colon_mode == 0 {
                err_invalid_option(argv, c);
            }
            let nc2 = *(&raw const NEXTCHAR);
            if *nc2 == 0 {
                *(&raw mut optind) += 1;
                *(&raw mut NEXTCHAR) = core::ptr::null();
            }
            return b'?' as i32;
        }

        if *p.add(1) == b':' {
            if *p.add(2) == b':' {
                // Optional argument
                let nc2 = *(&raw const NEXTCHAR);
                if *nc2 != 0 {
                    *(&raw mut optarg) = nc2 as *mut u8;
                    *(&raw mut NEXTCHAR) = core::ptr::null();
                } else {
                    *(&raw mut optarg) = core::ptr::null_mut();
                }
                *(&raw mut optind) += 1;
                *(&raw mut NEXTCHAR) = core::ptr::null();
            } else {
                // Required argument
                let nc2 = *(&raw const NEXTCHAR);
                if *nc2 != 0 {
                    *(&raw mut optarg) = nc2 as *mut u8;
                    *(&raw mut NEXTCHAR) = core::ptr::null();
                    *(&raw mut optind) += 1;
                } else if *(&raw const optind) + 1 < argc {
                    *(&raw mut optind) += 1;
                    *(&raw mut optarg) = *argv.offset(*(&raw const optind) as isize);
                    *(&raw mut optind) += 1;
                } else {
                    *(&raw mut optind) += 1;
                    *(&raw mut NEXTCHAR) = core::ptr::null();
                    if colon_mode != 0 {
                        return b':' as i32;
                    }
                    if *(&raw const opterr) != 0 {
                        err_requires_argument(argv, c);
                    }
                    return b'?' as i32;
                }
            }
        } else {
            let nc2 = *(&raw const NEXTCHAR);
            if *nc2 == 0 {
                *(&raw mut optind) += 1;
                *(&raw mut NEXTCHAR) = core::ptr::null();
            }
        }

        c as i32
    }
}

// -----------------------------------------------------------------------
// getopt_long
// -----------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getopt_long(
    argc: i32,
    argv: *const *mut u8,
    optstring: *const u8,
    longopts: *const option,
    longindex: *mut i32,
) -> i32 {
    unsafe { getopt_long_internal(argc, argv, optstring, longopts, longindex, 0) }
}

// -----------------------------------------------------------------------
// getopt_long_only
// -----------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getopt_long_only(
    argc: i32,
    argv: *const *mut u8,
    optstring: *const u8,
    longopts: *const option,
    longindex: *mut i32,
) -> i32 {
    unsafe { getopt_long_internal(argc, argv, optstring, longopts, longindex, 1) }
}
