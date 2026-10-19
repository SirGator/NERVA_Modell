/* Standalone VGA/PS2 environment. State lives exclusively in this guest.
 * A/D move a square across six cells; Space resets. No serial output or reward.
 */
.code16
.global _start
_start:
    ljmp $0, $start
start:
    cli
    xorw %ax, %ax
    movw %ax, %ds
    movw %ax, %ss
    movw $0x7c00, %sp
    cld
    sti
    movw $0x0013, %ax
    int $0x10
    movw $0xa000, %ax
    movw %ax, %es
    call draw
poll:
    movb $1, %ah
    int $0x16
    jz poll
    xorb %ah, %ah
    int $0x16
    cmpb $0x1e, %ah
    je left
    cmpb $0x20, %ah
    je right
    cmpb $0x39, %ah
    jne poll
    movb $2, position
    jmp redraw
left:
    cmpb $0, position
    je poll
    decb position
    jmp redraw
right:
    cmpb $5, position
    je poll
    incb position
redraw:
    call draw
    jmp poll
draw:
    pushaw
    xorw %di, %di
    xorw %ax, %ax
    movw $64000, %cx
    rep stosb
    /* Gray track and six gray cell markers. */
    movw $(140*320+30), %di
    movb $8, %al
    movw $240, %cx
    rep stosb
    movw $(144*320+36), %di
    movw $6, %bx
marker:
    movw $8, %cx
    rep stosb
    addw $32, %di
    decw %bx
    jnz marker
    /* White 16x16 square at the current guest-local position. */
    movzbw position, %ax
    imulw $40, %ax
    addw $(110*320+32), %ax
    movw %ax, %di
    movb $15, %al
    movw $16, %bx
row:
    movw $16, %cx
    rep stosb
    addw $(320-16), %di
    decw %bx
    jnz row
    popaw
    ret
position: .byte 2
.org 510
.word 0xaa55
