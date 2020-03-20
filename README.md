## most of the logic resides in runtime/modules/ls-biding

This module is meant for Web3 grant.

In this module, definex implemented a DeFi model which follows a 'maker-taker'.

Basically, there are 3 major roles:

    1. borrower: those who want to borrow money. they can publish their needs (collateral amount, borrow amount, how long they will repay, a specific interest rate, etc.) on the platform.

    2. loaner: those who bring liquidity to the platform. they select the borrows that most profitable, and lend the money to the borrower. By doing this, they earn the negotiated interest.

    3. liquidator: those who keep monitoring if there is any loan with a ltv lower than the 'LTVLiquidate'. By doing this, they would be rewarded.

## price if fed through offchain worker

runtime/modules/new-oracle

you can customize your crypto price sources by "add_source".
