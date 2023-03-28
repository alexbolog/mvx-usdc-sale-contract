#![no_std]

multiversx_sc::imports!();

/// A simple smart contract allowing the owner to create a package-style sale
/// specifying the USDC price of each given package, with support for any
/// token listed on xExchange (or similar DEX that implements `getEquivalent`
/// view, check the proxy definition).
#[multiversx_sc::contract]
pub trait UsdPriceTokenSaleContract {
    #[init]
    fn init(&self, opt_usdc_token_id: OptionalValue<TokenIdentifier>) {
        match opt_usdc_token_id {
            OptionalValue::Some(usdc_token_id) => {
                self.usdc_token_id().set(&usdc_token_id);
            },
            OptionalValue::None => {}
        };

        require!(!self.usdc_token_id().is_empty(), "USDC token id not specified");
    }

    #[payable("*")]
    #[endpoint(buy)]
    fn buy_tokens(&self, package_id: u8) {        
        self.validate_package_purchase(package_id);
        let caller = self.blockchain().get_caller();

        let payment = self.call_value().egld_or_single_esdt();
        let exchange_pool_proxy_address = self.get_proxy_address_or_fail(&payment.token_identifier);

        let usdc_token_id = self.usdc_token_id().get();
        let package_cost = self.package_prices(package_id).get();


        if &payment.token_identifier == &usdc_token_id {
            require!(&payment.amount == &package_cost, "invalid payment amount");
            self.send_package_content(package_id, &caller);
        } else {
            self.contract_proxy(exchange_pool_proxy_address)
                .get_equivalent(usdc_token_id, package_cost)
                .async_call()
                .with_callback(
                    self.callbacks()
                        .finish_transfer(
                            package_id,
                            &caller,
                            &payment.token_identifier,
                            payment.token_nonce,
                            &payment.amount
                        )
                )
                .call_and_exit();
        }
        
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
        let package_content = self.package_content(package_id).get();
        let out_vec = ManagedVec::from_single_item(package_content);
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
    #[endpoint(setPackageContent)]
    fn add_package_content(&self, package_id: u8, token_id: TokenIdentifier, nonce: u64, amount: BigUint) {
        let content = EsdtTokenPayment::new(token_id, nonce, amount);
        self.package_content(package_id).set(&content);
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

    #[only_owner]
    #[endpoint]
    fn withdraw(&self, token_id: EgldOrEsdtTokenIdentifier, nonce: u64, opt_receiver: OptionalValue<ManagedAddress>) {
        let receiver = match opt_receiver {
            OptionalValue::Some(val) => val,
            OptionalValue::None => self.blockchain().get_caller()
        };
        let balance = self.blockchain().get_sc_balance(&token_id, nonce);
        require!(&balance >= &BigUint::zero(), "nothing to withdraw");

        self.send()
            .direct(
                &receiver,
                &token_id,
                nonce,
                &balance
            );
    }

    fn get_proxy_address_or_fail(&self, token_id: &EgldOrEsdtTokenIdentifier) -> ManagedAddress {
        let proxy_address_storage = self.proxy_address(token_id);
        require!(!proxy_address_storage.is_empty(), "payment token not supported");
        return proxy_address_storage.get();
    }

    fn validate_package_purchase(&self, package_id: u8) {
        require!(!self.package_prices(package_id).is_empty(), "package price not set");
        let package_content_storage = self.package_content(package_id);
        require!(!package_content_storage.is_empty(), "package content not set");
        let package_content = package_content_storage.get();
        let sc_balance = self.blockchain().get_sc_balance(&EgldOrEsdtTokenIdentifier::esdt(package_content.token_identifier), package_content.token_nonce);
        require!(&sc_balance >= &package_content.amount, "not enough package content balance on the smart contract");
    }

    #[view(getUsdcTokenIdentifier)]
    #[storage_mapper("usdc_token_id")]
    fn usdc_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getUsdcPrice)]
    #[storage_mapper("usdc_costs")]
    fn package_prices(&self, package_id: u8) -> SingleValueMapper<BigUint>;

    #[view(getPackageContent)]
    #[storage_mapper("package_content")]
    fn package_content(&self, package_id: u8) -> SingleValueMapper<EsdtTokenPayment>;

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