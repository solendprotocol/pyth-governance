import { Keypair } from "@solana/web3.js";
import assert from "assert";
import { StakeConnection } from "../app/StakeConnection";
import {
  standardSetup,
  readAnchorConfig,
  getPortNumber,
  ANCHOR_CONFIG_PATH,
} from "./utils/before";
import {} from "../../staking/tests/utils/before";
import BN from "bn.js";
import path from "path";
import { expectFailApi } from "./utils/utils";
import { assertBalanceMatches } from "./utils/api_utils";

const portNumber = getPortNumber(path.basename(__filename));

describe("api", async () => {
  const pythMintAccount = new Keypair();
  const pythMintAuthority = new Keypair();

  let stakeConnection: StakeConnection;

  let controller;

  let EPOCH_DURATION;
  let owner;

  after(async () => {
    controller.abort();
  });

  before(async () => {
    const config = readAnchorConfig(ANCHOR_CONFIG_PATH);
    ({ controller, stakeConnection } = await standardSetup(
      portNumber,
      config,
      pythMintAccount,
      pythMintAuthority,
      null,
      1000
    ));

    EPOCH_DURATION = stakeConnection.config.epochDuration;
    owner = stakeConnection.program.provider.wallet.publicKey;
  });

  it("Deposit and lock", async () => {
    await stakeConnection.depositAndLockTokens(undefined, new BN(600));
  });

  it("Find and parse stake accounts", async () => {
    const res = await stakeConnection.getStakeAccounts(owner);

    assert.equal(res.length, 1);
    assert.equal(
      res[0].stakeAccountPositionsJs.owner.toBase58(),
      owner.toBase58()
    );
    assert.equal(
      res[0].stakeAccountMetadata.owner.toBase58(),
      owner.toBase58()
    );
    assert.equal(
      res[0].stakeAccountPositionsJs.positions[0].amount.toNumber(),
      600
    );
    assert.equal(res[0].tokenBalance.toNumber(), 600);
    await assertBalanceMatches(
      stakeConnection,
      owner,
      { locked: { locking: new BN(600) } },
      await stakeConnection.getTime()
    );

    await stakeConnection.depositAndLockTokens(res[0], new BN(100));

    const after = await stakeConnection.getStakeAccounts(owner);
    assert.equal(after.length, 1);
    assert.equal(
      after[0].stakeAccountPositionsJs.positions[1].amount.toNumber(),
      100
    );
    assert.equal(after[0].tokenBalance.toNumber(), 700);
    // No time has passed, but LOCKING tokens count as locked for the balance summary, so it shows as 700
    await assertBalanceMatches(
      stakeConnection,
      owner,
      { locked: { locking: new BN(700) } },
      await stakeConnection.getTime()
    );
  });

  it("Unlock too much", async () => {
    const res = await stakeConnection.getStakeAccounts(owner);
    const stakeAccount = res[0];

    await expectFailApi(
      stakeConnection.unlockTokens(stakeAccount, new BN(701)),
      "Amount greater than locked amount"
    );

    await assertBalanceMatches(
      stakeConnection,
      owner,
      { locked: { locking: new BN(700) } },
      await stakeConnection.getTime()
    );
  });

  it("Unlock", async () => {
    const res = await stakeConnection.getStakeAccounts(owner);
    const stakeAccount = res[0];

    await stakeConnection.unlockTokens(stakeAccount, new BN(600));

    await assertBalanceMatches(
      stakeConnection,
      owner,
      { locked: { locking: new BN(100) }, withdrawable: new BN(600) },
      await stakeConnection.getTime()
    );
  });

  it("Withdraw too much", async () => {
    const res = await stakeConnection.getStakeAccounts(owner);
    const stakeAccount = res[0];

    await expectFailApi(
      stakeConnection.withdrawTokens(stakeAccount, new BN(601)),
      "Amount exceeds withdrawable"
    );

    await assertBalanceMatches(
      stakeConnection,
      owner,
      { locked: { locking: new BN(100) }, withdrawable: new BN(600) },
      await stakeConnection.getTime()
    );
  });

  it("Withdraw", async () => {
    const res = await stakeConnection.getStakeAccounts(owner);
    const stakeAccount = res[0];
    await stakeConnection.withdrawTokens(stakeAccount, new BN(600));

    await assertBalanceMatches(
      stakeConnection,
      owner,
      { locked: { locking: new BN(100) } },
      await stakeConnection.getTime()
    );
  });
});
