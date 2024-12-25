<template>
    <div v-if="visible" :style="toastStyle" class="toast">
        <div class="toast-content">
            <span>{{ message }}</span>
            <button v-if="showClose" class="close-btn" @click="closeToast">X</button>
            <div v-if="showSpinner" class="spinner-overlay">
                <div v-if="isLoading" class="spinner"></div>
                <div v-else class="spinner-finished">✓</div>
            </div>
        </div>
    </div>
</template>

<script lang="ts">
import { ref, defineComponent, onMounted, Ref } from 'vue';

export default defineComponent({
    name: 'Toast',
    props: {
        message: {
            type: String,
            required: true,
        },
        type: {
            type: String,
            default: 'info', // 'info', 'success', 'error', etc.
            validator(value) {
                return typeof value === 'string'
                    && Object.values(['info', 'success', 'error']).includes(value)
            }
        },
        position: {
            type: String,
            default: 'bottom-right',
            validator(value) {
                return typeof value === 'string'
                    && Object.values([
                        'top-left',
                        'top',
                        'top-right',
                        'bottom-left',
                        'bottom',
                        'bottom-right'
                    ]).includes(value)
            }
        },
        duration: {
            type: Number,
            default: 3000, // milliseconds
        },
        decoration: {
            type: String,
            default: 'none',
            validator(value) {
                return typeof value === 'string'
                    && Object.values(['none', 'close', 'spinner']).includes(value)
            }
        },
        // Indicating whether the spinner should spin or stop spinning,
        // only useful when the `decoration` is set to `spinner`
        isLoading: {
            type: Boolean,
            default: false
        }
    },
    setup(props) {
        const visible = ref(true);
        const toastStyle: Ref<any> = ref({});
        const showClose = ref(props.decoration === 'close');
        const showSpinner = ref(props.decoration === 'spinner');

        // controls toast position
        switch (props.position) {
            case 'top-left':
                toastStyle.value = { top: '50px', left: '5px', transform: 'translateX(5px)' };
                break;
            case 'top':
                toastStyle.value = { top: '50px', left: '50%', transform: 'translateX(-50%)' };
                break;
            case 'top-right':
                toastStyle.value = { top: '50px', right: '5px', transform: 'translateX(-5px)' };
                break;
            case 'bottom-left':
                toastStyle.value = { bottom: '10px', left: '5px', transform: 'translateX(5px)' };
                break;
            case 'bottom':
                toastStyle.value = { bottom: '10px', left: '50%', transform: 'translateX(-50%)' };
                break;
            case 'bottom-right':
                toastStyle.value = { bottom: '10px', right: '5px', transform: 'translateX(-5px)' };
                break;
        }

        // controls toast color
        switch (props.type) {
            case 'info':
                toastStyle.value.backgroundColor = '#2196f3';
                break;
            case 'success':
                toastStyle.value.backgroundColor = '#4caf50';
                break;
            case 'error':
                toastStyle.value.backgroundColor = '#f44336';
                break;
        }

        // Automatically hide the toast after a certain duration
        onMounted(() => {
            if (props.duration > 0) {
                setTimeout(() => {
                    visible.value = false;
                }, props.duration);
            }
        });

        // Close the toast manually
        const closeToast = () => {
            visible.value = false;
        };

        return {
            visible,
            showClose,
            showSpinner,
            toastStyle,
            closeToast,
        };
    },
});
</script>

<style scoped>
.toast {
    background-color: #333;
    color: white;
    /** preserve space on the right for decorations */
    padding: 10px 6% 10px 10px;
    border-radius: 5px;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.2);
    min-height: 20px;
    min-width: 50px;
    font-size: 16px;
    position: fixed;
    z-index: 999;
}

.toast .close-btn {
    background: none;
    border: none;
    color: white;
    font-size: 16px;
    cursor: pointer;
    position: fixed;
    top: 50%;
    transform: translateY(-50%);
    right: 5px;
}

.toast .close-btn:hover {
    color: #ddd;
}

.spinner-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    display: flex;
    justify-content: right;
    align-items: center;
}

.spinner {
    width: 21px;
    height: 21px;
    display: inline-block;
    animation: spin 1s linear infinite;
    position: fixed;
    right: 10px;
    border: 3px solid rgba(255, 255, 255, 0.3);
    border-top: 3px solid #fff;
    border-style: solid;
    border-radius: 50%;
}

.spinner-finished {
    color: white;
    font-size: 20px;
    padding: 10px;
}

@keyframes spin {
    0% {
        transform: rotate(0deg);
    }

    100% {
        transform: rotate(360deg);
    }
}
</style>
