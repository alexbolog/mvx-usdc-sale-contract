#![no_std]

multiversx_sc::imports!();

const USDC_TOKEN_ID: &[u8] = b"USDC-c76f1f";
const WEGLD_TOKEN_ID: &[u8] = b"WEGLD-bd4d79";

/// A simple smart contract allowing the owner to create a package-style sale
/// specifying the USDC price of each given package, with support for any
/// token listed on xExchange (or similar DEX that implements `getEquivalent`
/// view, check the proxy definition).
#[multiversx_sc::contract]
pub trait UsdPriceTokenSaleContract {
    #[init]
    fn init(&self) {}

    #[payable("*")]
    #[endpoint(buyTokens)]
    fn buy_tokens(&self, package_id: u8) {
        
        self.validate_package_purchase(package_id);

        let payment = self.call_value().egld_or_single_esdt();
        let exchange_pool_proxy_address = self.proxy_address(&payment.token_identifier);
        require!(!exchange_pool_proxy_address.is_empty(), "payment currency not supported");

        let output_token_id = match payment.token_identifier.is_egld() {
            true => TokenIdentifier::from(ManagedBuffer::from(WEGLD_TOKEN_ID)),
            false => payment.token_identifier.clone().unwrap_esdt()
        };

        let package_cost = self.package_prices(package_id).get();

        self.contract_proxy(exchange_pool_proxy_address.get())
            .get_equivalent(output_token_id, package_cost)
            .async_call()
            .with_callback(
                self.callbacks()
                    .finish_transfer(
                        package_id,
                        &self.blockchain().get_caller(),
                        &payment.token_identifier,
                        payment.token_nonce,
                        &payment.amount
                    )
            )
            .call_and_exit();
    }

    #[callback]
    fn finish_transfer(
        &self,
        #[call_result] result: ManagedAsyncCallResult<BigUint>,
        package_id: u8,
        caller: &ManagedAddress,
        transfer_token: &EgldOrEsdtTokenIdentifier,
        transfer_token_nonce: u64,
        transfer_amount: &BigUint,
    ) {
        match result {
            ManagedAsyncCallResult::Ok(input_amount_needed) => {
                if transfer_amount >= &input_amount_needed {
                    let diff = *&transfer_amount - &input_amount_needed;
                    self.send_package_content(package_id, caller);
                    self.send()
                        .direct(
                            caller,
                            transfer_token,
                            transfer_token_nonce,
                            &diff
                        );
                } else {
                    self.send()
                        .direct(
                            caller, 
                            transfer_token,
                            transfer_token_nonce,
                            transfer_amount
                        );
                }
            },
            ManagedAsyncCallResult::Err(_) => {
                self.send()
                    .direct(
                        caller, 
                        transfer_token,
                        transfer_token_nonce,
                        transfer_amount
                    );
            }
        }
    }

    fn send_package_content(&self, package_id: u8, receiver: &ManagedAddress) {
        let mut out_vec = ManagedVec::new();
        for item in self.package_content(package_id).iter() {
            out_vec.push(item);
        }
        self.send()
            .direct_multi(receiver, &out_vec);
    }

    #[only_owner]
    #[endpoint(setProxyAddress)]
    fn set_proxy_address(&self, input_token: EgldOrEsdtTokenIdentifier, address: ManagedAddress) {
        self.proxy_address(&input_token).set(&address);
    }

    #[only_owner]
    #[endpoint(setPackagePrice)]
    fn set_package_price(&self, package_id: u8, usd_amount: BigUint) {
        self.package_prices(package_id).set(&usd_amount);
    }

    #[only_owner]
    #[endpoint(addPackageContent)]
    fn add_package_content(&self, package_id: u8, token_id: TokenIdentifier, nonce: u64, amount: BigUint) {
        let content = EsdtTokenPayment::new(token_id, nonce, amount);
        require!(self.package_content(package_id).insert(content), "package content already set");
    }

    #[only_owner]
    #[endpoint(removePackageContent)]
    fn remove_package_content(&self, package_id: u8, token_id: TokenIdentifier, nonce: u64, amount: BigUint) {
        let content = EsdtTokenPayment::new(token_id, nonce, amount);
        self.package_content(package_id).remove(&content);
    }

    #[only_owner]
    #[endpoint(removePackage)]
    fn remove_package(&self, package_id: u8) {
        self.package_prices(package_id).clear();
        self.package_content(package_id).clear();
    }

    #[only_owner]
    #[payable("*")]
    #[endpoint]
    fn deposit(&self) {}

    fn validate_package_purchase(&self, package_id: u8) {
        require!(!self.package_prices(package_id).is_empty(), "package price not set");
        require!(!self.package_content(package_id).is_empty(), "package content not set");
    }

    #[view(getUsdcPrice)]
    #[storage_mapper("usdc_costs")]
    fn package_prices(&self, package_id: u8) -> SingleValueMapper<BigUint>;

    #[view(getPackageContent)]
    #[storage_mapper("package_content")]
    fn package_content(&self, package_id: u8) -> SetMapper<EsdtTokenPayment>;

    #[view(getProxyAddress)]
    #[storage_mapper("proxy_address")]
    fn proxy_address(&self, input_token: &EgldOrEsdtTokenIdentifier) -> SingleValueMapper<ManagedAddress>;

    #[proxy]
    fn contract_proxy(&self, sc_address: ManagedAddress) -> callee_proxy::Proxy<Self::Api>;
}

mod callee_proxy {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait CalleeContract {
        #[view(getEquivalent)]
        fn get_equivalent(&self, token_in: TokenIdentifier, amount_in: BigUint) -> BigUint;
    }
}