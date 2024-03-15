;; Smart contract with a single `call` method that forwards arguments to the baby liminal chain extension.
;; The contract *ignores* the result from the extension.
(module
	(import "seal0" "call_chain_extension"
		(func $call_chain_extension (param i32 i32 i32 i32 i32) (result i32))
	)
	(import "seal0" "seal_input" (func $seal_input (param i32 i32)))
	(import "seal0" "seal_return" (func $seal_return (param i32 i32 i32)))
	(import "env" "memory" (memory 16 16))

	;; Bytes [0, 4)  (just i32) are representing the length of the input.
	;; Bytes [4, 38) are reserved for the input to be read by the $seal_input function
	;;   - 4  bytes for extension method id
	;;   - 32 bytes for verifying key hash
	;;   - 2  bytes for empty proof and empty public input
	(data (i32.const 0) "\26")

	;; Function for instantiating the contract.
	(func (export "deploy"))

  ;; Function for calling the contract.
	(func (export "call")
		(call $seal_input
		  (i32.const 4) ;; input_ptr     (read bytes starting from offset 4)
		  (i32.const 0) ;; input_len_ptr (the length of the input is written at bytes [0, 4))
    )

		(call $call_chain_extension
			(i32.load (i32.const 4))	;; chain extension id (first 4 bytes of the input)
			(i32.const 8)				      ;; input_ptr (the rest of the input - the actual arguments for the extension)
			(i32.const 34)          	;; input_len (the length of the actual arguments for the extension)
			(i32.const 0)             ;; output_ptr     (there will be no output, so doesn't matter)
			(i32.const 0)				      ;; output_len_ptr (there will be no output, so doesn't matter)
		)

    ;; ignore chain extension result (usually it will be `UnknownVerificationKeyIdentifier` or `IncorrectProof`),
    ;; but we don't care here
		drop

    ;; return Ok(())
		(call $seal_return (i32.const 0) (i32.const 0) (i32.const 0))
	)
)
