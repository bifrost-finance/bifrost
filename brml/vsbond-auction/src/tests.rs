#![cfg(test)]

use crate::mock::*;
use crate::*;
use frame_support::{assert_ok, dispatch::DispatchError};

#[test]
fn create_order_should_work() {

}

#[test]
fn create_order_by_origin_illegal_should_fail() {

}

#[test]
fn create_order_without_enough_currency_should_fail() {

}

#[test]
fn revoke_order_should_work() {

}

#[test]
fn revoke_order_by_origin_illegal_should_fail() {

}

#[test]
fn revoke_order_not_in_trade_should_fail() {

}

#[test]
fn clinch_order_should_work() {

}

#[test]
fn clinck_order_by_origin_illegal_should_fail() {

}

#[test]
fn clinch_order_not_in_trade_should_fail() {

}