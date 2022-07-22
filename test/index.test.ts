import { Guard } from '../src/guard';
import { Wallet } from '@ethersproject/wallet';

const GUARD_PK = '0x3b6b2589512955bcc0514cc4ee37b546098342b0ed8575f762ea677a71e4d051';

describe('', () => {
  const wallet = new Wallet(GUARD_PK);
  const guard = new Guard(wallet);
  const boostId = '0x1';
  const recipient = '0xeF8305E140ac520225DAf050e2f71d5fBcC543e7';

  it('run strategy', async () => {
    const result = await guard.run(boostId, recipient);
    expect(result).toMatchSnapshot();
  });

  it('issue claim', async () => {
    const claim = await guard.claim({
      boostId,
      recipient,
      amount: 1
    });
    expect(claim).toMatchSnapshot();
  });
});
