import { boosts } from '../job';

export default async function query() {
  console.log(Object.values(boosts['5']));
  return Object.values(boosts['5']);
}
