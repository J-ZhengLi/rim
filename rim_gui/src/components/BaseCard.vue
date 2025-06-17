<template>
    <div class="glass">
        <slot></slot>
    </div>
</template>

<style scoped>
.glass {
    padding: 2%;
    border-radius: 20px;
    background: rgba(255, 255, 255, .1);
    border: 2px solid transparent;
    box-shadow: 0 0 0 2px rgba(255, 255, 255, .6), 0 16px 32px rgba(0, 0, 0, .12);
    backdrop-filter: url(#frosted);
    -webkit-backdrop-filter: blur(25px);
    outline: 0;
}
</style>
