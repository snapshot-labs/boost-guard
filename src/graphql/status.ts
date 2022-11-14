import { address, coupon } from '../guard';
import strategies from '../strategies';
import { boosts } from '../job';

export default async function query(_parent, args) {
  const { boostId, recipient, chainId } = args;

  const boost = boosts[chainId][boostId];
  const amount = await strategies[boost.strategy.strategy](recipient, boost.strategy.params);

  let sig;
  if (amount !== '0') {
    sig = await coupon(boostId, recipient, amount, chainId);
  }

  return {
    boostId,
    recipient,
    guard: address,
    chainId,
    amount,
    sig
  };
}
