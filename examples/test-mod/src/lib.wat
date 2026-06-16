;; Simple WASM module for testing the mod system
(module
    (memory (export "memory") 1)
    
    ;; Update function called each frame
    (func (export "update") (param $dt f32)
        ;; Do nothing - just a placeholder
    )
)
