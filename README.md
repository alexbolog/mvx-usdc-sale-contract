# Package sale smart contract
A simple smart contract that facilitates the purchase of blockchain assets at a fixed USDC price.

## Features
- USDC based price
- payments in any supported token
- multiple packages allowed

## How it works
In order for the sale contract to work, the owner must define the following:
- at least one package (using an `u8` package_id) content
- the USD based price for each defined package
- at least one token to be used for payments (e.g. `EGLD` or `WEGLD-d7c6bb` for devnet)
- the USDC token identifier (`USDC-8d4068` for devnet)
- the token - USDC DEX pair contract to be used for fetching the relevant amount to be paid (e.g. the devnet xExchange USDC-WEGLD pair contract: `erd1qqqqqqqqqqqqqpgqq67uv84ma3cekpa55l4l68ajzhq8qm3u0n4s20ecvx`)

After the successful setup of the contract, the users can now purchase the defined package by sending the needed tokens amount to the contract and calling the `buy` function, followed by the package id in hex.

Example flow:
- the owner sets for sale a package of 1 $WEB token at a price of 1 USDC, and allows the payment in USDC and EGLD
- the owner sets the pool contract to be used for identifying the price and tops up the contract with the sold assets 
- for a price of 1 EGLD = 40$, 1 USDC would be 0.025 EGLD
- the user sends 0.03 EGLD to the smart contract and fills the data field with `buy@01`, where 01 is the ID of the package defined by the owner
- the user receives 1 $WEB token and 0.005 EGLD as remainder, while the contract keeps the required 0.025 EGLD representing user's payment


## Supported tokens
The token support is based on the DEX pools available on the network. In the examples above you can find xExchange being used, this is mainly due to the presence of high liquidity, as well as mainnet/testnet/devnet availability.
In order for the contract to work, the only requirements for a pair contract are:
- to be a ($YOURTOKEN, USDC) pool
- to have the `getEquivalent` view function implemented the same way as the proxy prototype in the contract

In theory, this call should work for xExchange's router mechanism, thus being able to get the price for virtually any token listed on the DEX.


## Endpoints
```
#[payable("*")]
#[endpoint(buy)]
fn buy_tokens(&self, package_id: u8)
```

Buy endpoint is the only public endpoint. Self-explanatory. User must send a slightly larger token amount than needed to account for possible price fluctuations and not have the transaction fail.


```
#[only_owner]
#[endpoint(setProxyAddress)]
fn set_proxy_address(&self, input_token: EgldOrEsdtTokenIdentifier, address: ManagedAddress)
```

This is used for defining the pool contract address to be used for fetching the price in the future.
- `input_token`: EGLD or any ESDT that is supported
- `address`: the pool contract to be queried when the purchase happens. If a pool contract is not defined for the given payment token, the transaction will fail.


```
#[only_owner]
#[endpoint(setPackagePrice)]
fn set_package_price(&self, package_id: u8, usd_amount: BigUint)
```

Sets the USD based selling price of a package. The USD price must have a denomination of 6 decimals. This means that 1 USD will be expressed as 1000000.


```
#[only_owner]
#[endpoint(setPackageContent)]
fn add_package_content(&self, package_id: u8, token_id: TokenIdentifier, nonce: u64, amount: BigUint)
```
Used for defining each package.
- `package_id`: self-explanatory
- `token_id`: any ESDT token identifier
- `nonce`: 0 for ESDT tokens, or the nonce of the asset in case of SFTs/NFTs
- `amount`: 1 for NFTs, otherwise as large as you want

```
#[only_owner]
#[endpoint(removePackage)]
fn remove_package(&self, package_id: u8) {
```
Self-explanatory. Removes the package content and price for the given id.


```
#[only_owner]
#[payable("*")]
#[endpoint]
fn deposit(&self) {}
```
Used for topping up the contract with the required assets. If this step is omitted, no `buy` transaction will be succes


```
#[only_owner]
#[endpoint]
fn withdraw(&self, token_id: EgldOrEsdtTokenIdentifier, nonce: u64, opt_receiver: OptionalValue<ManagedAddress>)
```
Used to withdraw any assets from the contract. The goal of this is:
- to retrieve the funds after successful payments
- to retrieve the for-sale assets after the sale is over


# Useful parameter values

## USDC <> WEGLD pair contracts 
- devnet: `erd1qqqqqqqqqqqqqpgqq67uv84ma3cekpa55l4l68ajzhq8qm3u0n4s20ecvx`
- mainnet: `erd1qqqqqqqqqqqqqpgqeel2kumf0r8ffyhth7pqdujjat9nx0862jpsg2pqaq`

## USDC token id:
- devnet: `USDC-8d4068`
- mainnet: `USDC-c76f1f`
