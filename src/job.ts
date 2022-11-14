import { getBoost, getNextBoostId, getStrategy } from '@snapshot-labs/boost';
import { getToken, sleep } from './utils';

export const boosts = {};

async function run(start: number, chainId: number) {
  const nextBoostId = await getNextBoostId(chainId);

  let i = start;
  while (i < nextBoostId) {
    const boost: any = await getBoost(i, chainId);
    boost.id = i;
    boost.chainId = chainId;

    let token: any = { address: boost.token };
    try {
      token = await getToken(boost.token, chainId);
      token.address = boost.token;
    } catch (e) {}
    boost.token = token;

    boost.strategy = {};
    try {
      new URL(boost.strategyURI);
      boost.strategy = await getStrategy(boost.strategyURI);
    } catch (e) {}

    if (!boosts[chainId]) boosts[chainId] = {};
    boosts[chainId][i.toString()] = boost;
    console.log('Boost', i, JSON.stringify(boost));
    i++;
  }

  await sleep(3e3);

  return run(i, chainId);
}

run(1, 5);
