import express from 'express';
import { Wallet } from '@ethersproject/wallet';
import { rpcSuccess } from './utils';
import { Guard } from './guard';

const privateKey = process.env.GUARD_PK || '';
const wallet = new Wallet(privateKey);
const guard = new Guard(wallet);

const router = express.Router();

router.post('/', async (req, res) => {
  const { id, params } = req.body;
  const { boostId, recipient, amount } = params;
  const result = await guard.claim({ boostId, recipient, amount });
  return rpcSuccess(res, result, id);
});

export default router;
