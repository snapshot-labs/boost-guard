import { BigNumber } from '@ethersproject/bignumber';

const BOOST_VERSION = process.env.BOOST_VERSION || '1';
const BOOST_CHAIN_ID = process.env.BOOST_CHAIN_ID || '1';
const BOOST_CONTRACT = process.env.BOOST_CONTRACT;

if (!BOOST_CONTRACT) throw new Error('BOOST_CONTRACT is not set');

export const domain = {
  name: 'boost',
  version: BOOST_VERSION,
  chainId: BOOST_CHAIN_ID,
  verifyingContract: BOOST_CONTRACT
};

export const claimTypes = {
  Claim: [
    { name: 'boostId', type: 'uint256' },
    { name: 'recipient', type: 'address' },
    { name: 'amount', type: 'uint256' }
  ]
};

export interface Claim {
  boostId: BigNumber;
  recipient: string;
  amount: BigNumber;
}
