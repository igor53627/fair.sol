// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";

interface IERC3156FlashBorrower {
    function onFlashLoan(
        address initiator,
        address token,
        uint256 amount,
        uint256 fee,
        bytes calldata data
    ) external returns (bytes32);
}

interface ILIQ {
    function name() external view returns (string memory);
    function symbol() external view returns (string memory);
    function decimals() external view returns (uint8);
    function totalSupply() external view returns (uint256);
    function balanceOf(address account) external view returns (uint256);
    function transfer(address to, uint256 amount) external returns (bool);
    function approve(address spender, uint256 amount) external returns (bool);
    function allowance(address owner, address spender) external view returns (uint256);
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
    function mint(uint256 amount) external returns (bool);
    function redeem(uint256 amount) external returns (bool);
    function usdc() external view returns (address);
    function maxFlashLoan(address token) external view returns (uint256);
    function flashFee(address token, uint256 amount) external view returns (uint256);
    function flashLoan(
        IERC3156FlashBorrower receiver,
        address token,
        uint256 amount,
        bytes calldata data
    ) external returns (bool);
}

contract MinimalBorrower is IERC3156FlashBorrower {
    bytes32 private constant CALLBACK_SUCCESS = keccak256("ERC3156FlashBorrower.onFlashLoan");
    
    function onFlashLoan(
        address,
        address,
        uint256,
        uint256,
        bytes calldata
    ) external pure override returns (bytes32) {
        return CALLBACK_SUCCESS;
    }
}

contract LIQHuffTest is Test {
    ILIQ public liq;
    MinimalBorrower public borrower;

    function setUp() public {
        bytes memory bytecode = hex"61053880600a3d393df35f3560e01c806306fdde03146100ad57806395d89b41146100e0578063313ce5671461011357806318160ddd1461011d57806370a0823114610127578063dd62ed3e1461013e578063a9059cbb1461016257806323b872dd146101ea578063095ea7b3146102da578063a0712d6814610301578063db006a751461036d5780633e413bee146103e7578063613255ab146103f0578063d9d98ce41461042a5780635cffe9de14610440575f5ffd5b5060203d5260036020527f4c4951000000000000000000000000000000000000000000000000000000000060405260603df35b5060203d5260036020527f4c4951000000000000000000000000000000000000000000000000000000000060405260603df35b5060063d5260203df35b505f543d5260203df35b506004353d52600160205260403d20543d5260203df35b50600435602435903d52600260205260403d203d5260205260403d20543d5260203df35b5060243560043533813d9190823d52600160205260403d208054926101965790918092116101a057919091039055506101a4565b91019055506101a4565b3d3dfd5b908160019190823d52600160205260403d208054926101d25790918092116101dc57919091039055506101e0565b91019055506101e0565b3d3dfd5b5060013d5260203df35b50604435600435602435813314610252578133903d52600260205260403d203d5260205260403d208054807fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff1461024b5782116102d6578290039055610253565b5050610253565b5b81813d9190823d52600160205260403d2080549261028057909180921161028a579190910390555061028e565b910190555061028e565b3d3dfd5b9190508160019190823d52600160205260403d208054926102be5790918092116102c857919091039055506102cc565b91019055506102cc565b3d3dfd5b5060013d5260203df35b3d3dfd5b5060243533600435903d52600260205260403d203d5260205260403d205560013d5260203df35b506004357f23b872dd000000000000000000000000000000000000000000000000000000003d5233600452306024528060445260203d60643d3d5f5af1156103695780335f5482015f55803d52600160205260403d2080548301905550505060013d5260203df35b3d3dfd5b506004358033803d52600160205260403d20805483811061039b5783900390555f548290035f55505061039f565b3d3dfd5b7fa9059cbb000000000000000000000000000000000000000000000000000000003d52336004528060245260203d60443d3d5f5af1156103e3575060013d5260203df35b3d3dfd5b505f3d5260203df35b506004353014610402573d3d5260203df35b7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff3d5260203df35b506004353014610438573d3dfd5b3d3d5260203df35b50602435301461044e573d3dfd5b60043560443581815f5482015f55803d52600160205260403d2080548301905550507f23e30c8b00000000000000000000000000000000000000000000000000000000608052336084523060a4528060c4523d60e4526080610104526064356004018035806101245280916020016101449190913760c40160203d8260803d885af1156104ff57503d517f439148f0bbc682ca079e46d6e2c2f0c1e3b820f1a291b069d8882abf8cf18dd914610503575b3d3dfd5b803d52600160205260403d20805483811061052b5783900390555f548290035f55505061052f565b3d3dfd5b60013d5260203df3";
        
        address deployed;
        assembly {
            deployed := create(0, add(bytecode, 0x20), mload(bytecode))
        }
        require(deployed != address(0), "Huff deployment failed");
        liq = ILIQ(deployed);
        
        borrower = new MinimalBorrower();
        
        vm.label(address(liq), "LIQ_Huff");
        vm.label(address(borrower), "MinimalBorrower");
    }

    function test_Huff_Name() public view {
        assertEq(liq.name(), "LIQ");
    }

    function test_Huff_Symbol() public view {
        assertEq(liq.symbol(), "LIQ");
    }

    function test_Huff_Decimals() public view {
        assertEq(liq.decimals(), 6);
    }

    function test_Huff_TotalSupply_Initially_Zero() public view {
        assertEq(liq.totalSupply(), 0);
    }

    function test_Huff_BalanceOf_Initially_Zero() public view {
        assertEq(liq.balanceOf(address(this)), 0);
    }

    function test_Huff_maxFlashLoan_ReturnsMax_ForLIQ() public view {
        uint256 maxLoan = liq.maxFlashLoan(address(liq));
        assertEq(maxLoan, type(uint256).max, "maxFlashLoan for LIQ should be max uint256");
    }

    function test_Huff_maxFlashLoan_ReturnsZero_ForOtherToken() public view {
        uint256 maxLoan = liq.maxFlashLoan(address(0x1234));
        assertEq(maxLoan, 0, "maxFlashLoan for other token should be zero");
    }

    function test_Huff_flashFee_ReturnsZero() public view {
        uint256 fee = liq.flashFee(address(liq), 1_000_000e18);
        assertEq(fee, 0, "flash fee should be zero");
    }

    function test_Huff_flashFee_Reverts_ForOtherToken() public {
        vm.expectRevert();
        liq.flashFee(address(0x1234), 1000e6);
    }

    function test_Huff_FlashMint_Success() public {
        uint256 balanceBefore = liq.balanceOf(address(borrower));
        uint256 supplyBefore = liq.totalSupply();
        
        bool success = liq.flashLoan(
            borrower,
            address(liq),
            1_000_000e6,
            ""
        );
        
        assertTrue(success, "flashLoan should return true");
        assertEq(liq.balanceOf(address(borrower)), balanceBefore, "borrower balance should be unchanged");
        assertEq(liq.totalSupply(), supplyBefore, "total supply should be unchanged");
    }

    function test_Huff_FlashMint_WrongToken_Reverts() public {
        vm.expectRevert();
        liq.flashLoan(
            borrower,
            address(0x1234),
            100e6,
            ""
        );
    }

    function test_Huff_GasBenchmark_FlashMint() public {
        uint256 gasBefore = gasleft();
        liq.flashLoan(
            borrower,
            address(liq),
            1_000_000e18,
            ""
        );
        uint256 gasUsed = gasBefore - gasleft();
        
        console.log("=== LIQ HUFF FLASH MINT GAS BENCHMARK ===");
        console.log("LIQ Huff Flash Mint Gas Used:", gasUsed);
        console.log("");
        console.log("=== COMPETITOR REFERENCE (from Jeiwan benchmarks) ===");
        console.log("Euler v1:                ~18,570 gas");
        console.log("Balancer v2:             ~24,407 gas");
        console.log("Aave v3:                 ~69,708 gas");
        console.log("dYdX (Solo Margin):      ~50,000 gas");
        console.log("");
        
        assertLt(gasUsed, 30000, "Flash mint should use <30k gas (excluding callback)");
    }

    function test_Huff_GasBenchmark_FlashMint_LargeAmount() public {
        uint256 gasBefore = gasleft();
        liq.flashLoan(
            borrower,
            address(liq),
            100_000_000_000e6,
            ""
        );
        uint256 gasUsed = gasBefore - gasleft();
        
        console.log("LIQ Huff Flash Mint (100B USDC equivalent):", gasUsed, "gas");
        console.log("  -> Balancer CANNOT serve this (limited to ~$50M pool liquidity)");
        console.log("  -> dYdX CANNOT serve this (limited to ~$30M)");
        console.log("  -> LIQ: UNLIMITED via flash MINT");
    }

    function test_Huff_Approve() public {
        bool success = liq.approve(address(0xBEEF), 1000e6);
        assertTrue(success);
        assertEq(liq.allowance(address(this), address(0xBEEF)), 1000e6);
    }
}
