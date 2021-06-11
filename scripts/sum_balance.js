const Big = require('big.js');
const fs = require('fs');
const path = require('path')

const GenesisConfigDir = '../node/service/res/genesis_config'
const BalanceDir = GenesisConfigDir + '/balances'
const VestingDir = GenesisConfigDir + '/vesting'

let balance,account,basename,account_key,accounts,
	accountBalances={},accountBalanceTotal={},vestingBalanceTotal={},
	totalBalances,liquid,locked,vestingBalances,vestingLocked,vestingLiquid;
fs.readdirSync(BalanceDir).forEach(file => {
	accounts = JSON.parse(fs.readFileSync(BalanceDir + '/' + file))
	basename = path.parse(file).name
	totalBalances = Big(0)
	if(accounts.balances){
		for(account of accounts.balances) {
			account_key = basename + ":" + account[0]
			balance = Big(account[1].toString())
			accountBalances[account_key]= balance
			totalBalances = totalBalances.plus(balance)
		}
		accountBalanceTotal[basename]={total:totalBalances.toString()}
	}
});

fs.readdirSync(VestingDir).forEach(file => {
	accounts = JSON.parse(fs.readFileSync(VestingDir + '/' + file))
	basename = path.parse(file).name
	vestingBalances = Big(0)
	vestingLocked = Big(0)
	vestingLiquid = Big(0)
	if(accounts.vesting){
		for(account of accounts.vesting) {
			account_key = basename + ":" + account[0]
			balance = accountBalances[account_key]
			liquid = Big(account[3].toString())
			locked = balance.minus(liquid)
			vestingBalances = vestingBalances.plus(balance)
			vestingLiquid = vestingLiquid.plus(liquid)
			vestingLocked = vestingLocked.plus(locked)
		}
		vestingBalanceTotal[basename]={total:vestingBalances.toString(),liquid:vestingLiquid.toString(),locked:vestingLocked.toString()}
	}
});

let result = {
	balance:accountBalanceTotal,
	vesting:vestingBalanceTotal
}
console.log(result)
fs.writeFileSync('./result.json', JSON.stringify(result,null,2));
