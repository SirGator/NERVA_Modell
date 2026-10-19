/* BIOS boot sector. Real x86 IN/OUT instructions, no host-side guest mock.
 * COM1: 0x3f8 data, 0x3fd line status. Debug: 0xe9, exit: 0xf4.
 * The task lives exclusively here: observation XOR 2 -> desired action.
 */
.code16
.global _start
_start:
    ljmp $0, $start
start:
    cli
    xorw %ax, %ax
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %ss
    movw $0x7c00, %sp
    cld
    /* 115200 baud, 8N1, polling, FIFO enabled. */
    movw $0x3f9, %dx
    outb %al, %dx
    movw $0x3fb, %dx
    movb $0x80, %al
    outb %al, %dx
    movw $0x3f8, %dx
    movb $1, %al
    outb %al, %dx
    incw %dx
    xorb %al, %al
    outb %al, %dx
    movw $0x3fb, %dx
    movb $3, %al
    outb %al, %dx
    movw $0x3fa, %dx
    movb $0xc7, %al
    outb %al, %dx
    /* Fresh model abstains; a restored model may already know this task. */
    xorb %bh, %bh
    xorb %bl, %bl
    call exchange
    cmpb $0xff, %al
    je begin_training
    cmpb $2, %al
    jne fail
begin_training:
    /* 32 epochs, four observations each. BH=operation, BL=observation. */
    movw $32, %bp
    movb $1, %bh
epoch:
    xorb %bl, %bl
train:
    call exchange
    incb %bl
    cmpb $4, %bl
    jne train
    decw %bp
    jnz epoch
    /* Frozen inference; replies must match the learned task. */
    xorb %bh, %bh
    xorb %bl, %bl
probe:
    call exchange
    movb %bl, %ah
    xorb $2, %ah
    cmpb %ah, %al
    jne fail
    incb %bl
    cmpb $4, %bl
    jne probe
    movb $0xff, %bh
    xorb %bl, %bl
    call exchange
    movw $pass_text, %si
    call print
    movb $0x10, %al
    jmp exit
fail:
    movw $fail_text, %si
    call print
    movb $0x11, %al
exit:
    outb %al, $0xf4
halt:
    hlt
    jmp halt

exchange:
    movb $0xa5, %al
    call send
    movb %bh, %al
    call send
    movb %bl, %al
    call send
    xorb %al, %al
    cmpb $1, %bh
    jne payload
    movb %bl, %al
    xorb $2, %al
payload:
    call send
    call receive
    cmpb $0x5a, %al
    jne fail
    call receive
    testb %al, %al
    jnz fail
    call receive
    movb %al, %ah
    call receive
    cmpb %bh, %al
    jne fail
    movb %ah, %al
    ret
send:
    pushw %ax
    movw $0x3fd, %dx
    movl $100000000, %ecx
send_wait:
    inb %dx, %al
    testb $0x20, %al
    jnz send_ready
    decl %ecx
    jnz send_wait
    jmp fail
send_ready:
    popw %ax
    movw $0x3f8, %dx
    outb %al, %dx
    ret
receive:
    movw $0x3fd, %dx
    movl $100000000, %ecx
receive_wait:
    inb %dx, %al
    testb $1, %al
    jnz receive_ready
    decl %ecx
    jnz receive_wait
    jmp fail
receive_ready:
    movw $0x3f8, %dx
    inb %dx, %al
    ret
print:
    lodsb
    testb %al, %al
    jz printed
    outb %al, $0xe9
    jmp print
printed:
    ret
pass_text: .asciz "NERVA_VM_PASS\n"
fail_text: .asciz "NERVA_VM_FAIL\n"
.org 510
.word 0xaa55
