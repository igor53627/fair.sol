# fair.sol

**Stablecoin with PoA ≈ 1.0** — fair liquidation mechanism that eliminates front-running and value extraction.

## The Problem

Current DeFi liquidation mechanisms have high Price of Anarchy (PoA ≈ 5.0):
- Front-runners extract 98%+ of profits
- Gas priority auctions waste resources
- Winner-takes-all concentrates value

## The Solution: Fair

Fair combines four mechanisms to achieve PoA ≈ 0.97:

| Mechanism | What it fixes |
|-----------|---------------|
| **IPFE hidden weights** | Removes information asymmetry |
| **Commit-reveal** | Removes front-running |
| **Random selection** | Removes gas priority advantage |
| **Keeper Pool 70/30** | Removes winner-takes-all |

## Simulation Results

```
Strategy              PoA    Front-runner  Concentration
─────────────────────────────────────────────────────────
Transparent          5.00      98.8%         100%
Noise-Based          5.11      98.1%         100%
IPFE Only            4.63      83.4%         87.1%
Fair 60/40           1.71      20.0%         28.7%
Fair 50/50           1.63      19.8%         27.1%
Keeper Pool 70/30    0.97      20.1%         14.0%
```

## How It Works

### 1. Hidden Scoring (IPFE)

Liquidation score uses hidden weights:
```
score = w1*collateralRatio + w2*volatility + w3*utilization + w4*age + w5*size
```
Weights are hidden via Inner Product Functional Encryption. Keepers can't predict exactly which CDPs are liquidatable.

### 2. Commit-Reveal

```
Block 0-10:   Keepers commit hash(cdpId, keeper, nonce)
Block 10-20:  Keepers reveal and prove CDP is liquidatable
Block 20+:    Execute liquidation, distribute profits
```
No front-running because commitments are hidden until reveal.

### 3. Keeper Pool Distribution

```
Liquidation profit = $1000

Protocol treasury:  $300 (30%)
Keeper pool:        $700 (70%)

5 keepers participated:
  Each gets: $700 / 5 = $140
```
Equal split eliminates competition incentive.

## Installation

```bash
forge install
```

## Usage

```solidity
import {Fair} from "src/Fair.sol";

// Deploy with IPFE instance and master public key
Fair fair = new Fair(ipfeAddress, mpk, skSum);

// Create CDP
fair.createCDP{value: 10 ether}(5000e18);

// Keeper: Start liquidation round
uint256 roundId = fair.startLiquidationRound(cdpId);

// Keeper: Commit (during blocks 0-10)
bytes32 commitment = keccak256(abi.encodePacked(cdpId, msg.sender, nonce));
fair.commit(roundId, commitment);

// Keeper: Reveal (during blocks 10-20)
fair.reveal(roundId, nonce, randomness);

// Anyone: Execute after block 20
fair.executeLiquidation(roundId);
```

## Run Simulation

```bash
cd simulation
cargo run --release
```

## Dependencies

- [ipfe.sol](https://github.com/igor53627/ipfe.sol) - Inner Product Functional Encryption

## References

- [RAI](https://reflexer.finance/) - Reflexive stablecoin with PID controller
- [Price of Anarchy](https://en.wikipedia.org/wiki/Price_of_anarchy) - Game theory concept
- [IPFE Paper](https://eprint.iacr.org/2015/017) - Abdalla et al. 2015

## License

MIT
