// Copyright (C) 2020 by definex.io

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! Tests for the module.

#![cfg(test)]

use super::*;
use crate::mock::{
    new_test_ext, next_asset_id, root, ExtBuilder, GenericAsset, Origin, System, Test, TestEvent,
};
use frame_support::{assert_noop, assert_ok};

#[test]
fn issuing_asset_units_to_issuer_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        let balance = 100;
        let asset_id = GenericAsset::next_asset_id();
        assert_ok!(GenericAsset::create(
            Origin::ROOT,
            balance,
            b"temp".to_vec(),
        ));
        assert_eq!(GenericAsset::free_balance(&asset_id, &root), balance);
    });
}

#[test]
fn issuing_with_next_asset_id_overflow_should_not_work() {
    ExtBuilder::default().build().execute_with(|| {
        NextAssetId::<Test>::put(u32::max_value());
        assert_noop!(
            GenericAsset::create(Origin::ROOT, 1, b"temp".to_vec(),),
            Error::<Test>::NoIdAvailable
        );
        assert_eq!(GenericAsset::next_asset_id(), u32::max_value());
    });
}

#[test]
fn querying_total_supply_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        let asset_id = GenericAsset::next_asset_id();
        assert_ok!(GenericAsset::create(Origin::ROOT, 100, b"temp".to_vec(),));
        assert_eq!(GenericAsset::free_balance(&asset_id, &root), 100);
        assert_ok!(GenericAsset::transfer(
            Origin::signed(root),
            asset_id,
            2,
            50
        ));
        assert_eq!(GenericAsset::free_balance(&asset_id, &root), 50);
        assert_eq!(GenericAsset::free_balance(&asset_id, &2), 50);
        assert_ok!(GenericAsset::transfer(Origin::signed(2), asset_id, 3, 31));
        assert_eq!(GenericAsset::free_balance(&asset_id, &root), 50);
        assert_eq!(GenericAsset::free_balance(&asset_id, &2), 19);
        assert_eq!(GenericAsset::free_balance(&asset_id, &3), 31);
        assert_ok!(GenericAsset::transfer(
            Origin::signed(root),
            asset_id,
            root,
            1
        ));
        assert_eq!(GenericAsset::free_balance(&asset_id, &root), 50);
    });
}

#[test]
fn transferring_amount_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        let asset_id = GenericAsset::next_asset_id();
        let free_balance = 100;
        assert_ok!(GenericAsset::create(
            Origin::ROOT,
            free_balance,
            b"temp".to_vec(),
        ));
        assert_eq!(GenericAsset::free_balance(&asset_id, &root), free_balance);
        assert_ok!(GenericAsset::transfer(
            Origin::signed(root),
            asset_id,
            2,
            40
        ));
        assert_eq!(GenericAsset::free_balance(&asset_id, &root), 60);
        assert_eq!(GenericAsset::free_balance(&asset_id, &2), 40);
    });
}

#[test]
fn transferring_amount_should_fail_when_transferring_more_than_free_balance() {
    ExtBuilder::default().build().execute_with(|| {
        let asset_id = GenericAsset::next_asset_id();
        assert_ok!(GenericAsset::create(Origin::ROOT, 100, b"temp".to_vec()));
        assert_noop!(
            GenericAsset::transfer(Origin::signed(root), asset_id, 2, 2000),
            Error::<Test>::InsufficientBalance
        );
    });
}

#[test]
fn transferring_less_than_one_unit_should_not_work() {
    ExtBuilder::default().build().execute_with(|| {
        let asset_id = GenericAsset::next_asset_id();
        assert_ok!(GenericAsset::create(Origin::ROOT, 100, b"temp".to_vec(),));
        assert_eq!(GenericAsset::free_balance(&asset_id, &root), 100);
        assert_noop!(
            GenericAsset::transfer(Origin::signed(root), asset_id, 2, 0),
            Error::<Test>::ZeroAmount
        );
    });
}

#[test]
fn self_transfer_should_fail() {
    ExtBuilder::default().build().execute_with(|| {
        let asset_id = GenericAsset::next_asset_id();
        let balance = 100;
        assert_ok!(GenericAsset::create(
            Origin::ROOT,
            balance,
            b"temp".to_vec(),
        ));
        let initial_free_balance = GenericAsset::free_balance(&asset_id, &root);
        assert_ok!(GenericAsset::transfer(
            Origin::signed(root),
            asset_id,
            root,
            10
        ));
        assert_eq!(
            GenericAsset::free_balance(&asset_id, &root),
            initial_free_balance
        );
    });
}

#[test]
fn transferring_more_units_than_total_supply_should_not_work() {
    ExtBuilder::default().build().execute_with(|| {
        let asset_id = GenericAsset::next_asset_id();
        assert_ok!(GenericAsset::create(Origin::ROOT, 100, b"temp".to_vec()));
        assert_eq!(GenericAsset::free_balance(&asset_id, &root), 100);
        assert_noop!(
            GenericAsset::transfer(Origin::signed(root), asset_id, 2, 101),
            Error::<Test>::InsufficientBalance
        );
    });
}

#[test]
fn total_balance_should_be_zero() {
    new_test_ext().execute_with(|| {
        assert_eq!(GenericAsset::total_balance(&0, &0), 0);
    });
}

#[test]
fn total_balance_should_be_equal_to_account_balance() {
    ExtBuilder::default().build().execute_with(|| {
        let asset_id = GenericAsset::next_asset_id();
        assert_ok!(GenericAsset::create(Origin::ROOT, 100, b"temp".to_vec(),));
        assert_eq!(GenericAsset::total_balance(&asset_id, &root), 100);
    });
}

#[test]
fn free_balance_should_only_return_account_free_balance() {
    let asset_id = 0;
    ExtBuilder::default()
        .next_asset_id(10)
        .symbols(vec![(asset_id, b"temp".to_vec())])
        .build()
        .execute_with(|| {
            let who = 123;
            let amount = 50;
            assert_ok!(GenericAsset::mint(
                Origin::signed(root),
                asset_id,
                who,
                amount
            ));
            GenericAsset::set_reserved_balance(&asset_id, &who, 70);
            assert_eq!(GenericAsset::free_balance(&asset_id, &who), 50);
        });
}

#[test]
fn total_balance_should_be_equal_to_sum_of_account_balance_and_free_balance() {
    let asset_id = 0;
    ExtBuilder::default()
        .next_asset_id(10)
        .symbols(vec![(asset_id, b"temp".to_vec())])
        .build()
        .execute_with(|| {
            let who = 123;
            let amount = 50;
            assert_ok!(GenericAsset::mint(
                Origin::signed(root),
                asset_id,
                who,
                amount
            ));
            GenericAsset::set_reserved_balance(&asset_id, &who, 70);
            assert_eq!(GenericAsset::total_balance(&asset_id, &who), 120);
        });
}

#[test]
fn reserved_balance_should_only_return_account_reserved_balance() {
    ExtBuilder::default()
        .next_asset_id(10)
        .symbols(vec![(0, b"temp".to_vec())])
        .build()
        .execute_with(|| {
            GenericAsset::set_reserved_balance(&0, &0, 70);
            assert_eq!(GenericAsset::reserved_balance(&0, &0), 70);
        });
}

#[test]
fn set_reserved_balance_should_add_balance_as_reserved() {
    let asset_id = 0;
    ExtBuilder::default()
        .next_asset_id(10)
        .symbols(vec![(asset_id, b"temp".to_vec())])
        .build()
        .execute_with(|| {
            let who = 123;
            GenericAsset::set_reserved_balance(&asset_id, &who, 70);
            assert_eq!(GenericAsset::reserved_balance(&asset_id, &who), 70);
        });
}

#[test]
fn set_free_balance_should_add_amount_as_free_balance() {
    let asset_id = 0;
    ExtBuilder::default()
        .next_asset_id(10)
        .symbols(vec![(asset_id, b"temp".to_vec())])
        .build()
        .execute_with(|| {
            let who = 123;
            GenericAsset::set_free_balance(&asset_id, &who, 50);
            assert_eq!(GenericAsset::free_balance(&asset_id, &who), 50);
        });
}

#[test]
fn reserve_should_moves_amount_from_balance_to_reserved_balance() {
    ExtBuilder::default()
        .next_asset_id(10)
        .symbols(vec![(0, b"temp".to_vec())])
        .build()
        .execute_with(|| {
            let who = 123;
            assert_ok!(GenericAsset::mint(Origin::signed(root), 0, who, 100));
            assert!(GenericAsset::reserve(&0, &who, 70).is_ok());
            assert_eq!(GenericAsset::free_balance(&0, &who), 30);
            assert_eq!(GenericAsset::reserved_balance(&0, &who), 70);
        });
}

#[test]
fn reserve_should_not_moves_amount_from_balance_to_reserved_balance() {
    let asset_id = 0;
    ExtBuilder::default()
        .next_asset_id(10)
        .symbols(vec![(asset_id, b"temp".to_vec())])
        .build()
        .execute_with(|| {
            let who = 123;
            assert_ok!(GenericAsset::mint(Origin::signed(root), asset_id, who, 100));
            assert_noop!(
                GenericAsset::reserve(&asset_id, &who, 120),
                Error::<Test>::InsufficientBalance
            );
            assert_eq!(GenericAsset::free_balance(&asset_id, &who), 100);
            assert_eq!(GenericAsset::reserved_balance(&asset_id, &who), 0);
        });
}

#[test]
fn create_reserved_should_create_a_default_account_with_the_balance_given() {
    ExtBuilder::default()
        .next_asset_id(10)
        .build()
        .execute_with(|| {
            let default_permission = PermissionLatest {
                update: Owner::Address(1),
                mint: Owner::Address(1),
                burn: Owner::Address(1),
            };
            let options = AssetOptions {
                initial_issuance: 500,
                permissions: default_permission,
            };
            let expected_total_issuance = 500;
            let created_asset_id = 9;
            let created_account_id = 0;
            assert_ok!(GenericAsset::create_reserved(
                Origin::ROOT,
                created_asset_id,
                options
            ));
            assert_eq!(
                <TotalIssuance<Test>>::get(created_asset_id),
                expected_total_issuance
            );
            assert_eq!(
                <FreeBalance<Test>>::get(&created_asset_id, &created_account_id),
                expected_total_issuance
            );
        });
}

#[test]
fn mint_should_throw_permission_error() {
    ExtBuilder::default().build().execute_with(|| {
        let origin = 1;
        let asset_id = 4;
        let to_account = 2;
        let amount = 100;
        assert_ok!(GenericAsset::create(Origin::ROOT, 0, b"temp".to_vec()));
        assert_noop!(
            GenericAsset::mint(Origin::signed(origin), asset_id, to_account, amount),
            Error::<Test>::NoMintPermission,
        );
    });
}

#[test]
fn mint_should_increase_asset() {
    let origin = 1;
    let asset_id = 11;
    let to_account = 2;
    let amount = 500;
    ExtBuilder::default()
        .next_asset_id(12)
        .symbols(vec![(asset_id, b"temp".to_vec())])
        .build()
        .execute_with(|| {
            assert_eq!(GenericAsset::next_asset_id(), 12);
            assert_ok!(GenericAsset::mint(
                Origin::signed(root),
                asset_id,
                to_account,
                amount
            ));
            assert_eq!(GenericAsset::free_balance(&asset_id, &to_account), amount);
        });
}

#[test]
fn burn_should_throw_permission_error() {
    let asset_id = 0;
    ExtBuilder::default()
        .next_asset_id(10)
        .symbols(vec![(asset_id, b"temp".to_vec())])
        .build()
        .execute_with(|| {
            let origin = 1;
            let to_account = 2;
            let amount = 10;
            assert_ok!(GenericAsset::mint(
                Origin::signed(root),
                asset_id,
                to_account,
                amount
            ));
            assert_noop!(
                GenericAsset::burn(Origin::signed(origin), asset_id, to_account, amount),
                Error::<Test>::NoBurnPermission,
            );
        });
}

#[test]
fn burn_should_burn_an_asset() {
    ExtBuilder::default().build().execute_with(|| {
        let asset_id = next_asset_id;
        let to_account = 2;
        let amount = 1000;
        let initial_issuance = 100;
        let burn_amount = 400;
        let expected_amount = 600;
        assert_ok!(GenericAsset::create(
            Origin::ROOT,
            initial_issuance,
            b"temp".to_vec(),
        ));
        assert_ok!(GenericAsset::mint(
            Origin::signed(root),
            asset_id,
            to_account,
            amount
        ));
        assert_ok!(GenericAsset::burn(
            Origin::signed(root),
            asset_id,
            to_account,
            burn_amount
        ));
        assert_eq!(
            GenericAsset::free_balance(&asset_id, &to_account),
            expected_amount
        );
    });
}

#[test]
fn check_permission_should_return_correct_permission() {
    ExtBuilder::default().build().execute_with(|| {
        let asset_id = next_asset_id;
        let initial_issuance = 100;
        assert_ok!(GenericAsset::create(
            Origin::ROOT,
            initial_issuance,
            b"temp".to_vec(),
        ));
        assert!(GenericAsset::check_permission(
            &asset_id,
            &root,
            &PermissionType::Burn
        ));
        assert!(GenericAsset::check_permission(
            &asset_id,
            &root,
            &PermissionType::Mint
        ));
        assert!(GenericAsset::check_permission(
            &asset_id,
            &root,
            &PermissionType::Update
        ));
    });
}

#[test]
fn check_permission_should_return_false_for_no_permission() {
    ExtBuilder::default().build().execute_with(|| {
        let origin = 1;
        let asset_id = next_asset_id;
        let initial_issuance = 100;
        assert_ok!(GenericAsset::create(
            Origin::ROOT,
            initial_issuance,
            b"temp".to_vec(),
        ));
        assert!(!GenericAsset::check_permission(
            &asset_id,
            &origin,
            &PermissionType::Burn
        ));
        assert!(!GenericAsset::check_permission(
            &asset_id,
            &origin,
            &PermissionType::Mint
        ));
        assert!(!GenericAsset::check_permission(
            &asset_id,
            &origin,
            &PermissionType::Update
        ));
    });
}

#[test]
fn update_permission_should_change_permission() {
    ExtBuilder::default().build().execute_with(|| {
        let origin = 1;
        let asset_id = next_asset_id;
        let initial_issuance = 100;
        let new_permission = PermissionLatest {
            update: Owner::Address(origin),
            mint: Owner::Address(origin),
            burn: Owner::None,
        };
        assert_ok!(GenericAsset::create(
            Origin::ROOT,
            initial_issuance,
            b"temp".to_vec(),
        ));
        assert_ok!(GenericAsset::update_permission(
            Origin::signed(root),
            asset_id,
            new_permission,
        ));
        assert!(GenericAsset::check_permission(
            &asset_id,
            &origin,
            &PermissionType::Mint
        ));
        assert!(!GenericAsset::check_permission(
            &asset_id,
            &origin,
            &PermissionType::Burn
        ));
    });
}

#[test]
fn update_permission_should_throw_error_when_lack_of_permissions() {
    ExtBuilder::default().build().execute_with(|| {
        let origin = 1;
        let asset_id = next_asset_id;
        let initial_issuance = 100;
        let new_permission = PermissionLatest {
            update: Owner::Address(origin),
            mint: Owner::Address(origin),
            burn: Owner::None,
        };
        assert_ok!(GenericAsset::create(
            Origin::ROOT,
            initial_issuance,
            b"temp".to_vec()
        ));
        assert_noop!(
            GenericAsset::update_permission(Origin::signed(origin), asset_id, new_permission),
            Error::<Test>::NoUpdatePermission,
        );
    });
}

#[test]
fn create_asset_works_with_given_asset_id_and_from_account() {
    ExtBuilder::default()
        .next_asset_id(10)
        .build()
        .execute_with(|| {
            let origin = 1;
            let from_account: Option<<Test as frame_system::Trait>::AccountId> = Some(1);
            let default_permission = PermissionLatest {
                update: Owner::Address(origin),
                mint: Owner::Address(origin),
                burn: Owner::Address(origin),
            };
            let expected_permission = PermissionVersions::V1(default_permission.clone());
            let asset_id = 9;
            let initial_issuance = 100;
            assert_ok!(GenericAsset::create_asset(
                Some(asset_id),
                from_account,
                AssetOptions {
                    initial_issuance: initial_issuance,
                    permissions: default_permission.clone()
                }
            ));
            assert_eq!(<NextAssetId<Test>>::get(), 10);
            assert_eq!(<TotalIssuance<Test>>::get(asset_id), initial_issuance);
            assert_eq!(
                <FreeBalance<Test>>::get(&asset_id, &origin),
                initial_issuance
            );
            assert_eq!(<Permissions<Test>>::get(&asset_id), expected_permission);
        });
}

#[test]
fn create_asset_with_non_reserved_asset_id_should_not_work() {
    ExtBuilder::default()
        .next_asset_id(10)
        .build()
        .execute_with(|| {
            let origin = 1;
            let from_account: Option<<Test as frame_system::Trait>::AccountId> = Some(1);

            let default_permission = PermissionLatest {
                update: Owner::Address(origin),
                mint: Owner::Address(origin),
                burn: Owner::Address(origin),
            };
            let asset_id = 11;
            let initial_issuance = 100;
            assert_noop!(
                GenericAsset::create_asset(
                    Some(asset_id),
                    from_account,
                    AssetOptions {
                        initial_issuance,
                        permissions: default_permission.clone()
                    }
                ),
                Error::<Test>::IdUnavailable,
            );
        });
}

#[test]
fn create_asset_with_a_taken_asset_id_should_not_work() {
    ExtBuilder::default().build().execute_with(|| {
        let origin = 1;
        let from_account: Option<<Test as frame_system::Trait>::AccountId> = Some(1);
        let default_permission = PermissionLatest {
            update: Owner::Address(origin),
            mint: Owner::Address(origin),
            burn: Owner::Address(origin),
        };
        let asset_id = GenericAsset::next_asset_id();
        let initial_issuance = 100;
        assert_ok!(GenericAsset::create_asset(
            None,
            from_account,
            AssetOptions {
                initial_issuance,
                permissions: default_permission.clone()
            }
        ));
        assert_noop!(
            GenericAsset::create_asset(
                Some(asset_id),
                from_account,
                AssetOptions {
                    initial_issuance,
                    permissions: default_permission.clone()
                }
            ),
            Error::<Test>::IdAlreadyTaken,
        );
    });
}

#[test]
fn create_asset_should_create_a_reserved_asset_when_from_account_is_none() {
    ExtBuilder::default().build().execute_with(|| {
        let origin = 1;
        let from_account: Option<<Test as frame_system::Trait>::AccountId> = None;
        let default_permission = PermissionLatest {
            update: Owner::Address(origin),
            mint: Owner::Address(origin),
            burn: Owner::Address(origin),
        };
        let created_account_id = 0;
        let asset_id = GenericAsset::next_asset_id();
        let initial_issuance = 100;

        assert_ok!(GenericAsset::create_asset(
            None,
            from_account,
            AssetOptions {
                initial_issuance: initial_issuance,
                permissions: default_permission
            }
        ));
        assert_eq!(
            <FreeBalance<Test>>::get(&asset_id, &created_account_id),
            initial_issuance
        );
    });
}

#[test]
fn create_asset_should_create_a_user_asset() {
    ExtBuilder::default().build().execute_with(|| {
        let origin = 1;
        let from_account: Option<<Test as frame_system::Trait>::AccountId> = None;
        let default_permission = PermissionLatest {
            update: Owner::Address(origin),
            mint: Owner::Address(origin),
            burn: Owner::Address(origin),
        };
        let created_account_id = 0;
        let reserved_asset_id = 100000;
        let initial_issuance = 100;
        let created_user_asset_id = next_asset_id;
        assert_ok!(GenericAsset::create_asset(
            None,
            from_account,
            AssetOptions {
                initial_issuance,
                permissions: default_permission,
            }
        ));
        assert_eq!(
            <FreeBalance<Test>>::get(&reserved_asset_id, &created_account_id),
            0
        );
        assert_eq!(
            <FreeBalance<Test>>::get(&created_user_asset_id, &created_account_id),
            initial_issuance
        );
        assert_eq!(
            <TotalIssuance<Test>>::get(created_user_asset_id),
            initial_issuance
        );
    });
}

#[test]
fn update_permission_should_raise_event() {
    let origin = 1;
    let initial_balance = 1000;
    let permissions = PermissionLatest {
        update: Owner::Address(origin),
        mint: Owner::Address(origin),
        burn: Owner::Address(origin),
    };

    ExtBuilder::default().build().execute_with(|| {
        let asset_id = next_asset_id;
        assert_ok!(GenericAsset::create(Origin::ROOT, 0, b"temp".to_vec(),));
        assert_ok!(GenericAsset::update_permission(
            Origin::signed(root),
            asset_id,
            permissions.clone()
        ));
        let expected_event =
            TestEvent::generic_asset(RawEvent::PermissionUpdated(asset_id, permissions.clone()));
        assert!(System::events()
            .iter()
            .any(|record| record.event == expected_event));
    });
}

#[test]
fn mint_should_raise_event() {
    let origin = 1;
    let initial_balance = 1000;
    let to = 2;
    let amount = 100;
    ExtBuilder::default().build().execute_with(|| {
        let asset_id = next_asset_id;
        assert_ok!(GenericAsset::create(Origin::ROOT, 0, b"temp".to_vec(),));
        assert_ok!(GenericAsset::mint(
            Origin::signed(root),
            asset_id,
            to,
            amount
        ));
        let expected_event = TestEvent::generic_asset(RawEvent::Minted(asset_id, to, amount));
        assert!(System::events()
            .iter()
            .any(|record| record.event == expected_event));
    });
}

#[test]
fn burn_should_raise_event() {
    let who = 1;
    let amount = 100;
    ExtBuilder::default().build().execute_with(|| {
        let asset_id = GenericAsset::next_asset_id();
        assert_ok!(GenericAsset::create(Origin::ROOT, amount, b"temp".to_vec()));
        assert_ok!(GenericAsset::transfer(
            Origin::signed(root),
            asset_id,
            who,
            amount
        ));
        assert_ok!(GenericAsset::burn(
            Origin::signed(root),
            asset_id,
            who,
            amount
        ));
        let expected_event = TestEvent::generic_asset(RawEvent::Burned(asset_id, who, amount));
        assert!(System::events()
            .iter()
            .any(|record| record.event == expected_event));
    });
}
