## most of the P2P logic resides in runtime/modules/ls-biding

This module is meant for Web3 grant.

In this module, definex implemented a DeFi model which follows a 'maker-taker'.

Basically, there are 3 major roles:

    1. borrower: those who want to borrow money. they can publish their needs (collateral amount, borrow amount, how long they will repay, a specific interest rate, etc.) on the platform.

    2. loaner: those who bring liquidity to the platform. they select the borrows that most profitable, and lend the money to the borrower. By doing this, they earn the negotiated interest.

    3. liquidator: those who keep monitoring if there is any loan with a ltv lower than the 'LTVLiquidate'. By doing this, they would be rewarded.

## price is fed through offchain worker

runtime/modules/new-oracle

you can customize your crypto price sources by "add_source".

## assets are based on runtime/modules/generic-asset

this is a modified version of frame/pallet-generic-asset.

we need every asset dynamically created can be reserved with a lock on the balance. but the default frame/pallet-generic-asset implementation doesn't support that.

## we are still adding more test cases
