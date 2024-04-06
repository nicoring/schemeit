(module
    (define fib (lambda (n) (cond ((< n 2) 1) (#t (+ (fib (- n 1)) (fib (- n 2)))))))
    (define range (lambda (n) (cond ((= n 0) #nil)(#t (cons n (range (- n 1)))))))
    (define null? (lambda (x) (= x #nil)))
    (define map (lambda (func l) (if (null? l) #nil (cons (func (car l)) (map func (cdr l))))))
    (define make-account
      (lambda (balance)
        (lambda (amt)
            (begin (set! balance (+ balance amt))
                    balance))))
)