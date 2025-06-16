<template>
    <div class="glass">
        <slot></slot>
    </div>
</template>

<style scoped>
.glass {
    position: fixed;
    inset: 50% auto auto 50%;
    transform: translate(-50%, -50%);
    border-radius: 20px;
    background: rgba(255, 255, 255, .08);
    border: 2px solid transparent;
    box-shadow: 0 0 0 2px rgba(255, 255, 255, .6), 0 16px 32px rgba(0, 0, 0, .12);
    backdrop-filter: url(#frosted);
    -webkit-backdrop-filter: blur(20px);
    outline: 0;
}
</style>
