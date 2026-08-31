#![cfg(all(test, feature = "server"))]

#[path = "http_tests/support.rs"]
mod support;
pub(crate) use support::*;
#[path = "http_tests/fixtures.rs"]
mod fixtures;
pub(crate) use fixtures::*;

#[path = "http_tests/case_00.rs"]
mod case_00;
#[path = "http_tests/case_01.rs"]
mod case_01;
#[path = "http_tests/case_02.rs"]
mod case_02;
#[path = "http_tests/case_03.rs"]
mod case_03;
#[path = "http_tests/case_04.rs"]
mod case_04;
#[path = "http_tests/case_05.rs"]
mod case_05;
#[path = "http_tests/case_06.rs"]
mod case_06;
#[path = "http_tests/case_07.rs"]
mod case_07;
#[path = "http_tests/case_08.rs"]
mod case_08;
#[path = "http_tests/case_09.rs"]
mod case_09;
#[path = "http_tests/case_10.rs"]
mod case_10;
#[path = "http_tests/case_11.rs"]
mod case_11;
#[path = "http_tests/case_12.rs"]
mod case_12;
#[path = "http_tests/case_13.rs"]
mod case_13;
#[path = "http_tests/case_14.rs"]
mod case_14;
#[path = "http_tests/case_15.rs"]
mod case_15;
#[path = "http_tests/case_16.rs"]
mod case_16;
#[path = "http_tests/case_17.rs"]
mod case_17;
#[path = "http_tests/case_18.rs"]
mod case_18;
#[path = "http_tests/case_19.rs"]
mod case_19;
#[path = "http_tests/case_20.rs"]
mod case_20;
#[path = "http_tests/case_21.rs"]
mod case_21;
#[path = "http_tests/case_22.rs"]
mod case_22;
#[path = "http_tests/case_23.rs"]
mod case_23;
#[path = "http_tests/case_24.rs"]
mod case_24;
#[path = "http_tests/case_25.rs"]
mod case_25;
#[path = "http_tests/case_26.rs"]
mod case_26;
#[path = "http_tests/case_27.rs"]
mod case_27;
#[path = "http_tests/case_28.rs"]
mod case_28;
#[path = "http_tests/case_29.rs"]
mod case_29;
#[path = "http_tests/case_30.rs"]
mod case_30;
#[path = "http_tests/case_31.rs"]
mod case_31;
#[path = "http_tests/case_32.rs"]
mod case_32;
#[path = "http_tests/case_33.rs"]
mod case_33;
#[path = "http_tests/case_34.rs"]
mod case_34;
#[path = "http_tests/case_35.rs"]
mod case_35;
#[path = "http_tests/case_36.rs"]
mod case_36;
#[path = "http_tests/case_37.rs"]
mod case_37;
#[path = "http_tests/case_38.rs"]
mod case_38;
#[path = "http_tests/case_39.rs"]
mod case_39;
#[path = "http_tests/case_40.rs"]
mod case_40;
#[path = "http_tests/case_41.rs"]
mod case_41;
#[path = "http_tests/case_42.rs"]
mod case_42;

#[path = "http_tests/packages.rs"]
mod packages;
