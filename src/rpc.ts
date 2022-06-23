import express from 'express';
import { Wallet } from '@ethersproject/wallet';
import { rpcSuccess } from './utils';
import { Guard } from './guard';
import { BigNumber } from '@ethersproject/bignumber';

const privateKey = process.env.GUARD_PK || '';
const wallet = new Wallet(privateKey);
const guard = new Guard(wallet);

const router = express.Router();

router.post('/', async (req, res) => {
  const { id, params } = req.body;
  const { boostId, recipient, amount } = params;

  const claim = {
    boostId: BigNumber.from(boostId),
    recipient,
    amount: BigNumber.from(amount)
  };

  const result = await guard.claim(claim);
  return rpcSuccess(res, result, id);
});

export default router;
