import { boosts } from '../job';

export default async function query(_parent, args) {
  const { id, chainId } = args;

  return boosts[chainId][id];
}
