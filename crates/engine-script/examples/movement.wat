;; A simple WASM module that demonstrates ECS interaction.
;;
;; This module imports host functions from the "env" namespace and
;; implements a movement system that updates Position components
;; based on Velocity.
;;
;; The module uses a flat binary layout for components:
;; - Position: 3x f32 (x, y, z) = 12 bytes
;; - Velocity: 3x f32 (x, y, z) = 12 bytes

(module
  ;; Import host functions
  (import "env" "spawn" (func $spawn (result i32)))
  (import "env" "despawn" (func $despawn (param i32)))
  (import "env" "has_component" (func $has_component (param i32 i32 i32) (result i32)))
  (import "env" "get_component" (func $get_component (param i32 i32 i32 i32 i32) (result i32)))
  (import "env" "set_component" (func $set_component (param i32 i32 i32 i32 i32) (result i32)))
  (import "env" "add_component" (func $add_component (param i32 i32 i32 i32 i32) (result i32)))
  (import "env" "component_size" (func $component_size (param i32 i32) (result i32)))
  (import "env" "log" (func $log (param i32 i32)))
  (import "env" "delta_time" (func $delta_time (result f32)))

  ;; Memory: 1 page (64 KiB) initial, max 256 pages (16 MiB)
  (memory (export "memory") 1 256)

  ;; String constants in linear memory
  (data (i32.const 0) "Position")    ;; 8 bytes at offset 0
  (data (i32.const 16) "Velocity")   ;; 8 bytes at offset 16
  (data (i32.const 32) "WASM movement system initialized") ;; 33 bytes at offset 32

  ;; Buffer for component data (at offset 256)
  ;; Position buffer: 12 bytes at offset 256
  ;; Velocity buffer: 12 bytes at offset 268

  ;; Helper: read f32 from linear memory
  (func $read_f32 (param $ptr i32) (result f32)
    (f32.load (local.get $ptr))
  )

  ;; Helper: write f32 to linear memory
  (func $write_f32 (param $ptr i32) (param $val f32)
    (f32.store (local.get $ptr) (local.get $val))
  )

  ;; Exported update function called by the host each tick
  (func (export "update") (param $dt f32)
    (local $entity i32)
    (local $pos_x f32) (local $pos_y f32) (local $pos_z f32)
    (local $vel_x f32) (local $vel_y f32) (local $vel_z f32)
    (local $i i32)
    (local $has_vel i32)

    ;; For this example, we'll process entities 0-99
    ;; In a real module, you'd get the entity list from the host
    (local.set $i (i32.const 0))

    (block $break
      (loop $loop
        ;; Break if i >= 100
        (br_if $break (i32.ge_u (local.get $i) (i32.const 100)))

        ;; Check if entity has Velocity
        (local.set $has_vel
          (call $has_component
            (local.get $i)    ;; entity
            (i32.const 16)    ;; "Velocity" name ptr
            (i32.const 8)     ;; "Velocity" name len
          )
        )

        (if (i32.ne (local.get $has_vel) (i32.const 0))
          (then
            ;; Check if entity has Position
            (if (i32.ne
              (call $has_component
                (local.get $i)    ;; entity
                (i32.const 0)     ;; "Position" name ptr
                (i32.const 8)     ;; "Position" name len
              ) (i32.const 0))
              (then
                ;; Read Position into buffer at offset 256
                (drop (call $get_component
                  (local.get $i)    ;; entity
                  (i32.const 0)     ;; "Position" name ptr
                  (i32.const 8)     ;; "Position" name len
                  (i32.const 256)   ;; result buffer ptr
                  (i32.const 12)    ;; result buffer capacity (3 * f32)
                ))

                ;; Read Velocity into buffer at offset 268
                (drop (call $get_component
                  (local.get $i)    ;; entity
                  (i32.const 16)    ;; "Velocity" name ptr
                  (i32.const 8)     ;; "Velocity" name len
                  (i32.const 268)   ;; result buffer ptr
                  (i32.const 12)    ;; result buffer capacity (3 * f32)
                ))

                ;; Load position values
                (local.set $pos_x (call $read_f32 (i32.const 256)))
                (local.set $pos_y (call $read_f32 (i32.const 260)))
                (local.set $pos_z (call $read_f32 (i32.const 264)))

                ;; Load velocity values
                (local.set $vel_x (call $read_f32 (i32.const 268)))
                (local.set $vel_y (call $read_f32 (i32.const 272)))
                (local.set $vel_z (call $read_f32 (i32.const 276)))

                ;; Update position: pos += vel * dt
                (local.set $pos_x (f32.add (local.get $pos_x) (f32.mul (local.get $vel_x) (local.get $dt))))
                (local.set $pos_y (f32.add (local.get $pos_y) (f32.mul (local.get $vel_y) (local.get $dt))))
                (local.set $pos_z (f32.add (local.get $pos_z) (f32.mul (local.get $vel_z) (local.get $dt))))

                ;; Write updated position back to buffer
                (call $write_f32 (i32.const 256) (local.get $pos_x))
                (call $write_f32 (i32.const 260) (local.get $pos_y))
                (call $write_f32 (i32.const 264) (local.get $pos_z))

                ;; Set the updated Position component
                (drop (call $set_component
                  (local.get $i)    ;; entity
                  (i32.const 0)     ;; "Position" name ptr
                  (i32.const 8)     ;; "Position" name len
                  (i32.const 256)   ;; value buffer ptr
                  (i32.const 12)    ;; value buffer len (3 * f32)
                ))
              )
            )
          )
        )

        ;; Increment i
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)
      )
    )
  )

  ;; Exported init function (optional, called once on load)
  (func (export "init")
    (call $log (i32.const 32) (i32.const 33))
  )
)
