<script setup lang="ts">
import AuthAside from '@/components/auth/AuthAside.vue'
import InputElement from '@/components/ui/InputElement.vue'
import { api } from '@/lib/axios'
import { useAuthStore } from '@/stores/auth'
import { toTypedSchema } from '@vee-validate/zod'
import { WebcamIcon } from 'lucide-vue-next'
import { useField, useForm } from 'vee-validate'
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import * as z from 'zod'

const router = useRouter()
const loading = ref(false)

const { handleSubmit, setFieldError } = useForm({
  validationSchema: toTypedSchema(
    z.object({
      username: z
        .string()
        .min(3, 'Username must be at least 3 characters')
        .max(32, 'Username must be at most 32 characters'),
      email: z.string().min(1, 'Email is required').email('Please enter a valid email address'),
      password: z.string().min(1, 'Password is required'),
    }),
  ),
})

const { value: username, errorMessage: usernameError } = useField<string>('username')
const { value: email, errorMessage: emailError } = useField<string>('email')
const { value: password, errorMessage: passwordError } = useField<string>('password')

const onSubmit = handleSubmit(async (values) => {
  if (loading.value) return
  loading.value = true
  try {
    await api.post('/user/', {
      username: values.username,
      email: values.email,
      password: values.password,
    })

    const res = await api.post('/auth/signin', {
      email: values.email,
      password: values.password,
    })

    const auth = useAuthStore()
    const token = res.data?.token
    if (token) {
      auth.setAccessToken(token)
    } else {
      setFieldError('email', 'Unexpected server response')
      return
    }

    await router.push('/')
  } catch (err) {
    const axiosErr = err as { response?: { data?: { error?: string }; status?: number } }
    const message = axiosErr.response?.data?.error || 'Something went wrong'
    if (message === 'username already exists') {
      setFieldError('username', message)
    } else if (message === 'email already exists' || axiosErr.response?.status === 409) {
      setFieldError('email', message)
    } else if (axiosErr.response?.status === 422) {
      setFieldError('username', message)
      setFieldError('email', message)
      setFieldError('password', message)
    } else {
      setFieldError('email', message)
    }
  } finally {
    loading.value = false
  }
})
</script>

<template>
  <main id="main" class="min-h-screen w-full bg-gray-100 p-4 sm:p-8 text-gray-900">
    <div
      class="mx-auto grid min-h-[calc(100vh-2rem)] sm:min-h-[calc(100vh-4rem)] max-w-5xl grid-cols-1 overflow-hidden bg-white lg:grid-cols-2 rounded-4xl"
    >
      <AuthAside />

      <!-- Right panel -->
      <section class="flex flex-col justify-between p-6 sm:p-10">
        <div></div>

        <div class="mx-auto w-full max-w-sm">
          <div class="text-center">
            <h1 class="text-2xl font-semibold tracking-tight">Create your account</h1>
            <p class="mt-2 text-sm text-slate-600">Enter your email below to create your account</p>
          </div>

          <form class="mt-6 flex flex-col gap-3" @submit="onSubmit" novalidate>
            <InputElement
              v-model="username"
              label="Username"
              type="text"
              autocomplete="username"
              :error="usernameError"
            />

            <InputElement
              v-model="email"
              label="Email"
              type="email"
              autocomplete="email"
              :error="emailError"
            />

            <InputElement
              v-model="password"
              label="password"
              type="password"
              autocomplete="password"
              :error="passwordError"
            />

            <button
              type="submit"
              :disabled="loading"
              :aria-busy="loading"
              class="mt-2 h-11 w-full rounded-lg bg-primary text-sm font-semibold text-white transition hover:bg-primary-hover active:scale-[0.99] disabled:opacity-60 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
            >
              {{ loading ? 'Signing up...' : 'Sign Up' }}
            </button>
          </form>

          <p class="mt-6 text-center text-sm text-slate-700">
            Already have an account?
            <a href="/signin" class="font-semibold text-slate-900 hover:underline">Sign In</a>
          </p>
        </div>

        <div class="mt-10 flex justify-between text-xs text-slate-500">
          <span>Copyright 2026</span>
        </div>
      </section>
    </div>
  </main>
</template>
