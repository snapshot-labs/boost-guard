import { Wallet } from '@ethersproject/wallet';
import { BOOST_ADDRESS } from '@snapshot-labs/boost';

const privateKey = process.env.GUARD_PK || '';
const wallet = new Wallet(privateKey);

export const address = wallet.getAddress();

const domain = {
  name: 'boost',
  version: '1',
  chainId: 5,
  verifyingContract: BOOST_ADDRESS
};

const claimTypes = {
  Claim: [
    { name: 'boostId', type: 'uint256' },
    { name: 'recipient', type: 'address' },
    { name: 'amount', type: 'uint256' }
  ]
};

export async function coupon(boostId, recipient, amount, chainId: number): Promise<string> {
  domain.chainId = chainId;
  const message = { boostId, recipient, amount };

  return await wallet._signTypedData(domain, claimTypes, message);
}
