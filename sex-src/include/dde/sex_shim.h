#ifndef DDE_SEX_SHIM_H
#define DDE_SEX_SHIM_H

#include <stdint.h>
#include <stddef.h>

/* --- Linux Kernel Type Shims --- */
typedef uint32_t gfp_t;
typedef uint64_t sector_t;

struct device {
    char name[32];
    void (*release)(struct device *dev);
};

struct resource {
    uint64_t start;
    uint64_t end;
    const char *name;
    uint64_t flags;
};

struct pci_dev {
    struct device dev;
    uint16_t vendor;
    uint16_t device;
    uint8_t bus;
    uint8_t devfn;
    struct resource resource[6];
};

struct bio_vec {
    void *bv_page;
    uint32_t bv_len;
    uint32_t bv_offset;
};

struct bio {
    uint16_t bi_vcnt;
    struct bio_vec *bi_io_vec;
    uint64_t bi_iter_sector;
};

typedef struct {
    volatile uint32_t locked;
} spinlock_t;

/* --- DDE-Sex Function Shims (Implemented in dde.rs) --- */

void *kmalloc(size_t size, int flags);
void kfree(void *ptr);

void *dma_alloc_coherent(struct device *dev, size_t size, uint64_t *dma_handle, int flags);
void dma_free_coherent(struct device *dev, size_t size, void *vaddr, uint64_t dma_handle);
uint64_t dma_map_single(struct device *dev, void *ptr, size_t size, int dir);

int request_irq(unsigned int irq, uint64_t (*handler)(uint64_t), unsigned long flags, const char *name, void *dev);

void spin_lock_init(spinlock_t *lock);
void _raw_spin_lock(spinlock_t *lock);
void _raw_spin_unlock(spinlock_t *lock);
uint64_t _raw_spin_lock_irqsave(spinlock_t *lock);
void _raw_spin_unlock_irqrestore(spinlock_t *lock, uint64_t flags);

void submit_bio(struct bio *bio_ptr);

int drm_ioctl(int fd, uint64_t request, void *arg);
ssize_t input_read(int fd, void *buffer, size_t count);

/* --- Memory Cache Shims --- */
void *find_get_page(uint64_t index);
void mark_page_dirty(void *page_ptr);
int set_page_dirty(void *page_ptr);

#endif /* DDE_SEX_SHIM_H */
