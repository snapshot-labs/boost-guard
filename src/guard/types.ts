import { BigNumber } from "@ethersproject/bignumber";

export const domain = {
  name: 'boost',
  version: '1',
  chainId: 4,
  verifyingContract: '0x0000000000000000000000000000000000000000'
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
