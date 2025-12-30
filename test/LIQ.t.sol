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
    function allowance(address owner, address spender) external view returns (uint256);
    function approve(address spender, uint256 amount) external returns (bool);
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
    function mint(uint256 amount) external returns (bool);
    function redeem(uint256 amount) external returns (bool);
    function asset() external view returns (address);
    function maxFlashLoan(address token) external view returns (uint256);
    function flashFee(address token, uint256 amount) external view returns (uint256);
    function flashLoan(
        IERC3156FlashBorrower receiver,
        address token,
        uint256 amount,
        bytes calldata data
    ) external returns (bool);
}

contract MockUSDC {
    string public name = "USD Coin";
    string public symbol = "USDC";
    uint8 public decimals = 6;
    uint256 public totalSupply;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    function mint(address to, uint256 amount) external {
        balanceOf[to] += amount;
        totalSupply += amount;
    }

    function approve(address spender, uint256 amount) external returns (bool) {
        allowance[msg.sender][spender] = amount;
        return true;
    }

    function transfer(address to, uint256 amount) external returns (bool) {
        require(balanceOf[msg.sender] >= amount, "insufficient balance");
        balanceOf[msg.sender] -= amount;
        balanceOf[to] += amount;
        return true;
    }

    function transferFrom(address from, address to, uint256 amount) external returns (bool) {
        require(balanceOf[from] >= amount, "insufficient balance");
        require(allowance[from][msg.sender] >= amount, "insufficient allowance");
        allowance[from][msg.sender] -= amount;
        balanceOf[from] -= amount;
        balanceOf[to] += amount;
        return true;
    }
}

contract MockFlashBorrower is IERC3156FlashBorrower {
    enum Mode { SUCCESS, REVERT, WRONG_RETURN }
    
    Mode public mode;
    address public lastInitiator;
    address public lastToken;
    uint256 public lastAmount;
    uint256 public lastFee;
    bytes public lastData;
    
    bytes32 public constant CALLBACK_SUCCESS = keccak256("ERC3156FlashBorrower.onFlashLoan");

    function setMode(Mode _mode) external {
        mode = _mode;
    }

    function onFlashLoan(
        address initiator,
        address token,
        uint256 amount,
        uint256 fee,
        bytes calldata data
    ) external override returns (bytes32) {
        lastInitiator = initiator;
        lastToken = token;
        lastAmount = amount;
        lastFee = fee;
        lastData = data;

        if (mode == Mode.REVERT) {
            revert("MockFlashBorrower: callback failed");
        }
        
        if (mode == Mode.WRONG_RETURN) {
            return bytes32(uint256(0xdead));
        }

        return CALLBACK_SUCCESS;
    }
}

contract MockLIQ is ILIQ {
    string public constant name = "LIQ";
    string public constant symbol = "LIQ";
    uint8 public constant decimals = 6;
    
    uint256 public totalSupply;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;
    
    address public immutable asset;
    
    bytes32 private constant CALLBACK_SUCCESS = keccak256("ERC3156FlashBorrower.onFlashLoan");
    
    constructor(address _asset) {
        asset = _asset;
    }
    
    function transfer(address to, uint256 amount) external returns (bool) {
        require(balanceOf[msg.sender] >= amount, "insufficient balance");
        balanceOf[msg.sender] -= amount;
        balanceOf[to] += amount;
        return true;
    }
    
    function approve(address spender, uint256 amount) external returns (bool) {
        allowance[msg.sender][spender] = amount;
        return true;
    }
    
    function transferFrom(address from, address to, uint256 amount) external returns (bool) {
        if (msg.sender != from) {
            uint256 allowed = allowance[from][msg.sender];
            if (allowed != type(uint256).max) {
                require(allowed >= amount, "insufficient allowance");
                allowance[from][msg.sender] = allowed - amount;
            }
        }
        require(balanceOf[from] >= amount, "insufficient balance");
        balanceOf[from] -= amount;
        balanceOf[to] += amount;
        return true;
    }
    
    function mint(uint256 amount) external returns (bool) {
        MockUSDC(asset).transferFrom(msg.sender, address(this), amount);
        totalSupply += amount;
        balanceOf[msg.sender] += amount;
        return true;
    }
    
    function redeem(uint256 amount) external returns (bool) {
        require(balanceOf[msg.sender] >= amount, "insufficient balance");
        balanceOf[msg.sender] -= amount;
        totalSupply -= amount;
        MockUSDC(asset).transfer(msg.sender, amount);
        return true;
    }
    
    function maxFlashLoan(address token) external view returns (uint256) {
        return token == address(this) ? type(uint256).max : 0;
    }
    
    function flashFee(address token, uint256) external view returns (uint256) {
        require(token == address(this), "unsupported token");
        return 0;
    }
    
    function flashLoan(
        IERC3156FlashBorrower receiver,
        address token,
        uint256 amount,
        bytes calldata data
    ) external returns (bool) {
        require(token == address(this), "unsupported token");
        
        totalSupply += amount;
        balanceOf[address(receiver)] += amount;
        
        bytes32 result = receiver.onFlashLoan(msg.sender, token, amount, 0, data);
        require(result == CALLBACK_SUCCESS, "callback failed");
        
        require(balanceOf[address(receiver)] >= amount, "insufficient repayment");
        balanceOf[address(receiver)] -= amount;
        totalSupply -= amount;
        
        return true;
    }
}

contract LIQTest is Test {
    MockLIQ public liq;
    MockUSDC public usdc;
    MockFlashBorrower public borrower;
    
    address alice = address(0xA11CE);
    address bob = address(0xB0B);

    function setUp() public {
        usdc = new MockUSDC();
        liq = new MockLIQ(address(usdc));
        borrower = new MockFlashBorrower();
        
        usdc.mint(alice, 1_000_000e6);
        usdc.mint(bob, 1_000_000e6);
        usdc.mint(address(liq), 10_000_000e6);
        
        vm.label(address(liq), "LIQ");
        vm.label(address(usdc), "USDC");
        vm.label(address(borrower), "MockFlashBorrower");
        vm.label(alice, "alice");
        vm.label(bob, "bob");
    }

    function test_FlashMint_Success() public {
        uint256 balanceBefore = liq.balanceOf(address(borrower));
        uint256 supplyBefore = liq.totalSupply();
        
        borrower.setMode(MockFlashBorrower.Mode.SUCCESS);
        
        bool success = liq.flashLoan(
            borrower,
            address(liq),
            100e6,
            abi.encode("test data")
        );
        
        assertTrue(success, "flashLoan should return true");
        assertEq(liq.balanceOf(address(borrower)), balanceBefore, "borrower balance should be unchanged");
        assertEq(liq.totalSupply(), supplyBefore, "total supply should be unchanged");
        assertEq(borrower.lastAmount(), 100e6, "callback should receive correct amount");
        assertEq(borrower.lastFee(), 0, "fee should be zero");
    }

    function test_FlashMint_WrongToken_Reverts() public {
        borrower.setMode(MockFlashBorrower.Mode.SUCCESS);
        
        vm.expectRevert("unsupported token");
        liq.flashLoan(
            borrower,
            address(usdc),
            100e6,
            ""
        );
    }

    function test_FlashMint_CallbackFails_Reverts() public {
        borrower.setMode(MockFlashBorrower.Mode.REVERT);
        
        vm.expectRevert("MockFlashBorrower: callback failed");
        liq.flashLoan(
            borrower,
            address(liq),
            100e6,
            ""
        );
    }

    function test_FlashMint_WrongReturnValue_Reverts() public {
        borrower.setMode(MockFlashBorrower.Mode.WRONG_RETURN);
        
        vm.expectRevert("callback failed");
        liq.flashLoan(
            borrower,
            address(liq),
            100e6,
            ""
        );
    }

    function test_maxFlashLoan_ReturnsMax_ForLIQ() public view {
        uint256 maxLoan = liq.maxFlashLoan(address(liq));
        assertEq(maxLoan, type(uint256).max, "maxFlashLoan for LIQ should be max uint256");
    }

    function test_maxFlashLoan_ReturnsZero_ForOtherToken() public view {
        uint256 maxLoan = liq.maxFlashLoan(address(usdc));
        assertEq(maxLoan, 0, "maxFlashLoan for other token should be zero");
    }

    function test_flashFee_ReturnsZero() public view {
        uint256 fee = liq.flashFee(address(liq), 1_000_000e18);
        assertEq(fee, 0, "flash fee should be zero");
    }

    function test_flashFee_Reverts_ForOtherToken() public {
        vm.expectRevert("unsupported token");
        liq.flashFee(address(usdc), 1000e6);
    }

    function test_Mint_Success() public {
        uint256 mintAmount = 1000e6;
        
        vm.startPrank(alice);
        usdc.approve(address(liq), mintAmount);
        
        uint256 usdcBefore = usdc.balanceOf(alice);
        uint256 liqBefore = liq.balanceOf(alice);
        
        bool success = liq.mint(mintAmount);
        
        assertTrue(success, "mint should return true");
        assertEq(usdc.balanceOf(alice), usdcBefore - mintAmount, "USDC should be transferred from alice");
        assertEq(liq.balanceOf(alice), liqBefore + mintAmount, "LIQ should be minted to alice");
        assertEq(liq.totalSupply(), mintAmount, "total supply should increase");
        vm.stopPrank();
    }

    function test_Redeem_Success() public {
        uint256 mintAmount = 1000e6;
        
        vm.startPrank(alice);
        usdc.approve(address(liq), mintAmount);
        liq.mint(mintAmount);
        
        uint256 redeemAmount = 500e6;
        uint256 usdcBefore = usdc.balanceOf(alice);
        uint256 liqBefore = liq.balanceOf(alice);
        uint256 supplyBefore = liq.totalSupply();
        
        bool success = liq.redeem(redeemAmount);
        
        assertTrue(success, "redeem should return true");
        assertEq(liq.balanceOf(alice), liqBefore - redeemAmount, "LIQ should be burned from alice");
        assertEq(usdc.balanceOf(alice), usdcBefore + redeemAmount, "USDC should be transferred to alice");
        assertEq(liq.totalSupply(), supplyBefore - redeemAmount, "total supply should decrease");
        vm.stopPrank();
    }

    function test_GasBenchmark_FlashMint() public {
        borrower.setMode(MockFlashBorrower.Mode.SUCCESS);
        
        uint256 gasBefore = gasleft();
        liq.flashLoan(
            borrower,
            address(liq),
            1_000_000e18,
            ""
        );
        uint256 gasUsed = gasBefore - gasleft();
        
        console.log("MockLIQ Flash Mint Gas Used:", gasUsed);
        console.log("Balancer v2 Flash Mint (reference): ~28,500");
        console.log("Aave v3 Flash Loan (reference): ~70,000");
    }

    function test_ERC20_Name() public view {
        assertEq(liq.name(), "LIQ");
    }

    function test_ERC20_Symbol() public view {
        assertEq(liq.symbol(), "LIQ");
    }

    function test_ERC20_Decimals() public view {
        assertEq(liq.decimals(), 6);
    }

    function test_ERC20_Transfer() public {
        vm.startPrank(alice);
        usdc.approve(address(liq), 1000e6);
        liq.mint(1000e6);
        
        bool success = liq.transfer(bob, 500e6);
        assertTrue(success);
        assertEq(liq.balanceOf(alice), 500e6);
        assertEq(liq.balanceOf(bob), 500e6);
        vm.stopPrank();
    }

    function test_ERC20_Approve() public {
        vm.prank(alice);
        bool success = liq.approve(bob, 1000e6);
        assertTrue(success);
        assertEq(liq.allowance(alice, bob), 1000e6);
    }

    function test_ERC20_TransferFrom() public {
        vm.startPrank(alice);
        usdc.approve(address(liq), 1000e6);
        liq.mint(1000e6);
        liq.approve(bob, 500e6);
        vm.stopPrank();
        
        vm.prank(bob);
        bool success = liq.transferFrom(alice, bob, 500e6);
        assertTrue(success);
        assertEq(liq.balanceOf(alice), 500e6);
        assertEq(liq.balanceOf(bob), 500e6);
    }
}
